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

use crate::storage::blob_tracker::{
    cast_delete_vote, delete_blob, list_all_blobs, record_peer_seed, remove_delete_vote,
    upsert_blob, BlobRecord, DELETE_VOTE_QUORUM,
};
use crate::storage::db::Database;

pub const SYNC_ALPN: &[u8] = b"/p2pstud/sync/1";

/// Catalog synchronization messages exchanged between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Full catalog sync containing all workspace records known to the node.
    CatalogDump(Vec<BlobRecord>),
    /// Notification that a new file was uploaded to the workspace.
    FileAdded(BlobRecord),
    /// Notification that a file was removed from the workspace (by owner or quorum).
    FileDeleted {
        hash: String,
        deleter_node_id: String,
    },
    /// A peer voted to mark a file for deletion.
    FileVoteDelete {
        hash: String,
        voter_node_id: String,
    },
    /// A peer retracted their vote to delete a file.
    FileUnvoteDelete {
        hash: String,
        voter_node_id: String,
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
    pub fn handle_message(&self, msg: SyncMessage, sender_peer_id: Option<&str>) -> Result<Option<SyncMessage>> {
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
                    for voter in r.delete_votes {
                        let _ = cast_delete_vote(&self.db, &r.hash, &voter);
                    }
                    if let Some(peer_id) = sender_peer_id {
                        let _ = record_peer_seed(&self.db, &r.hash, peer_id);
                    }
                    if let Some(owner) = &r.owner_node_id {
                        let _ = record_peer_seed(&self.db, &r.hash, owner);
                    }
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
                for voter in r.delete_votes {
                    let _ = cast_delete_vote(&self.db, &r.hash, &voter);
                }
                if let Some(peer_id) = sender_peer_id {
                    let _ = record_peer_seed(&self.db, &r.hash, peer_id);
                }
                if let Some(owner) = &r.owner_node_id {
                    let _ = record_peer_seed(&self.db, &r.hash, owner);
                }
                Ok(None)
            }
            SyncMessage::FileDeleted { hash, .. } => {
                let _ = delete_blob(&self.db, &hash);
                Ok(None)
            }
            SyncMessage::FileVoteDelete {
                hash,
                voter_node_id,
            } => {
                if let Ok(count) = cast_delete_vote(&self.db, &hash, &voter_node_id) {
                    if count >= DELETE_VOTE_QUORUM {
                        tracing::info!(
                            hash = %hash,
                            votes = count,
                            "Community quorum reached: purging consensus-deleted file"
                        );
                        let _ = delete_blob(&self.db, &hash);
                    }
                }
                Ok(None)
            }
            SyncMessage::FileUnvoteDelete {
                hash,
                voter_node_id,
            } => {
                let _ = remove_delete_vote(&self.db, &hash, &voter_node_id);
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
            let sender_peer = iroh::endpoint::get_remote_node_id(&connection).ok().map(|id| id.to_string());
            let (mut send, mut recv) = connection.accept_bi().await?;

            // Read incoming message with 2MB limit (plenty for thousands of catalog entries)
            let bytes = recv.read_to_end(2 * 1024 * 1024).await?;
            if let Ok(msg) = serde_json::from_slice::<SyncMessage>(&bytes) {
                if let Ok(Some(reply)) = this.handle_message(msg, sender_peer.as_deref()) {
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
    let connection = endpoint.connect(peer_addr.clone(), SYNC_ALPN).await?;
    let (mut send, mut recv) = connection.open_bi().await?;

    let my_records = list_all_blobs(db)?;
    let msg = SyncMessage::CatalogDump(my_records);
    let bytes = serde_json::to_vec(&msg)?;

    send.write_all(&bytes).await?;
    let _ = send.finish();

    // Read the remote catalog response
    if let Ok(reply_bytes) = recv.read_to_end(2 * 1024 * 1024).await {
        if let Ok(SyncMessage::CatalogDump(remote_records)) =
            serde_json::from_slice::<SyncMessage>(&reply_bytes)
        {
            let peer_id_str = peer_addr.node_id.to_string();
            for r in remote_records {
                let _ = upsert_blob(
                    db,
                    &r.hash,
                    r.name.as_deref(),
                    r.size_bytes,
                    false,
                    r.owner_node_id.as_deref(),
                );
                for voter in r.delete_votes {
                    let _ = cast_delete_vote(db, &r.hash, &voter);
                }
                let _ = record_peer_seed(db, &r.hash, &peer_id_str);
                if let Some(owner) = &r.owner_node_id {
                    let _ = record_peer_seed(db, &r.hash, owner);
                }
            }
        }
    }

    Ok(())
}

/// Broadcast a catalog mutation event to all actively connected swarm peers.
pub async fn broadcast_catalog_event(endpoint: &Endpoint, msg: SyncMessage) {
    let bytes = match serde_json::to_vec(&msg) {
        Ok(b) => b,
        Err(_) => return,
    };

    let remotes: Vec<_> = endpoint.remote_info_iter().collect();
    for remote in remotes {
        let peer_addr = NodeAddr::new(remote.node_id);
        let ep = endpoint.clone();
        let payload = bytes.clone();
        tokio::spawn(async move {
            if let Ok(conn) = ep.connect(peer_addr, SYNC_ALPN).await {
                if let Ok((mut send, _)) = conn.open_bi().await {
                    let _ = send.write_all(&payload).await;
                    let _ = send.finish();
                }
            }
        });
    }
}
