// src-tauri/src/p2p/sync.rs
// Workspace catalog synchronization protocol across swarm peers over QUIC.
// ALPN: /p2pstud/sync/1

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use iroh::endpoint::{Connecting, Endpoint, NodeAddr};
use iroh::protocol::ProtocolHandler;
use serde::{Deserialize, Serialize};

use crate::storage::blob_tracker::{delete_blob, list_all_blobs, upsert_blob, BlobRecord};
use crate::storage::db::Database;

pub const SYNC_ALPN: &[u8] = b"/p2pstud/sync/1";

/// Catalog synchronization messages exchanged between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Full catalog sync containing all workspace records known to the node.
    CatalogDump(Vec<BlobRecord>),
    /// Notification that a new file was uploaded to the workspace.
    FileAdded(BlobRecord),
    /// Notification that a file was removed from the workspace.
    FileDeleted {
        hash: String,
        deleter_node_id: String,
    },
}

/// Protocol handler registered on the Iroh Router for incoming catalog sync streams.
#[derive(Debug, Clone)]
pub struct CatalogSyncHandler {
    db: Arc<Database>,
}

impl CatalogSyncHandler {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Process an incoming message and update local SQLite catalog.
    pub fn handle_message(&self, msg: SyncMessage) -> Result<Option<SyncMessage>> {
        match msg {
            SyncMessage::CatalogDump(remote_records) => {
                for r in remote_records {
                    // Upsert remote metadata into local SQLite catalog (unpinned by default)
                    let _ = upsert_blob(
                        &self.db,
                        &r.hash,
                        r.name.as_deref(),
                        r.size_bytes,
                        false, // peer metadata starts unpinned locally until user pins or downloads
                        r.owner_node_id.as_deref(),
                    );
                }
                // Respond with our own catalog dump
                let my_records = list_all_blobs(&self.db).unwrap_or_default();
                Ok(Some(SyncMessage::CatalogDump(my_records)))
            }
            SyncMessage::FileAdded(r) => {
                let _ = upsert_blob(
                    &self.db,
                    &r.hash,
                    r.name.as_deref(),
                    r.size_bytes,
                    false,
                    r.owner_node_id.as_deref(),
                );
                Ok(None)
            }
            SyncMessage::FileDeleted { hash, .. } => {
                let _ = delete_blob(&self.db, &hash);
                Ok(None)
            }
        }
    }
}

impl ProtocolHandler for CatalogSyncHandler {
    fn accept(
        self: Arc<Self>,
        conn: Connecting,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            let connection = conn.await?;
            let (mut send, mut recv) = connection.accept_bi().await?;

            // Read incoming message with 2MB limit (plenty for thousands of catalog entries)
            let bytes = recv.read_to_end(2 * 1024 * 1024).await?;
            if let Ok(msg) = serde_json::from_slice::<SyncMessage>(&bytes) {
                if let Ok(Some(reply)) = this.handle_message(msg) {
                    if let Ok(reply_bytes) = serde_json::to_vec(&reply) {
                        send.write_all(&reply_bytes).await?;
                    }
                }
            }
            let _ = send.finish();
            Ok(())
        })
    }
}

/// Perform a bi-directional catalog exchange with a remote peer.
pub async fn sync_catalog_with_peer(
    endpoint: &Endpoint,
    db: &Arc<Database>,
    peer_addr: NodeAddr,
) -> Result<()> {
    let connection = endpoint.connect(peer_addr, SYNC_ALPN).await?;
    let (mut send, mut recv) = connection.open_bi().await?;

    let my_records = list_all_blobs(db)?;
    let msg = SyncMessage::CatalogDump(my_records);
    let bytes = serde_json::to_vec(&msg)?;

    send.write_all(&bytes).await?;
    send.finish()?;

    let reply_bytes = recv.read_to_end(2 * 1024 * 1024).await?;
    if let Ok(SyncMessage::CatalogDump(remote_records)) =
        serde_json::from_slice::<SyncMessage>(&reply_bytes)
    {
        for r in remote_records {
            let _ = upsert_blob(
                db,
                &r.hash,
                r.name.as_deref(),
                r.size_bytes,
                false,
                r.owner_node_id.as_deref(),
            );
        }
    }

    Ok(())
}

/// Broadcast a single catalog mutation (e.g. file added or deleted) to all active remotes.
pub async fn broadcast_catalog_event(
    endpoint: &Endpoint,
    msg: SyncMessage,
) {
    let remotes: Vec<_> = endpoint.remote_info_iter().collect();
    let payload = match serde_json::to_vec(&msg) {
        Ok(p) => p,
        Err(_) => return,
    };

    for remote in remotes {
        let ep = endpoint.clone();
        let payload = payload.clone();
        let node_id = remote.node_id;
        tokio::spawn(async move {
            let addr = NodeAddr::new(node_id);
            if let Ok(connection) = ep.connect(addr, SYNC_ALPN).await {
                if let Ok((mut send, mut recv)) = connection.open_bi().await {
                    let _ = send.write_all(&payload).await;
                    let _ = send.finish();
                    let _ = recv.read_to_end(1024).await;
                }
            }
        });
    }
}

