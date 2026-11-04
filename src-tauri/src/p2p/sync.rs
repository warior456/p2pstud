// src-tauri/src/p2p/sync.rs
// Workspace catalog synchronization protocol across swarm peers over QUIC.
// ALPN: /p2pstud/sync/1

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use iroh::endpoint::{Connecting, Endpoint, NodeAddr};
use iroh::protocol::ProtocolHandler;
use iroh_blobs::store::Map;
use serde::{Deserialize, Serialize};

use crate::storage::blob_tracker::{
    cast_delete_vote, cast_keep_vote, delete_blob, get_blob, list_all_blobs, record_peer_seed,
    remove_delete_vote, remove_keep_vote, remove_peer_seed, rename_blob, upsert_blob, BlobRecord,
};
use crate::storage::db::Database;

pub const DEFAULT_SYNC_ALPN: &[u8] = b"/p2pstud/sync/1";
pub const SYNC_ALPN: &[u8] = b"/p2pstud/sync/1";

/// Compute the QUIC ALPN for catalog sync based on the workspace secret.
/// If no secret is configured, uses DEFAULT_SYNC_ALPN.
/// If a secret is set, derives a unique 16-hex ALPN suffix to isolate the swarm from other networks.
pub fn get_sync_alpn(db: &Database) -> Vec<u8> {
    if let Ok(Some(secret)) = db.get_setting("network_secret") {
        let trimmed = secret.trim();
        if !trimmed.is_empty() {
            let hash = iroh_blobs::Hash::new(format!("p2pstud-net-secret:{trimmed}").as_bytes());
            let short = &hash.to_hex()[..16];
            return format!("/p2pstud/sync/1/{short}").into_bytes();
        }
    }
    DEFAULT_SYNC_ALPN.to_vec()
}

fn is_valid_blob_hash(h: &str) -> bool {
    h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()) && h.parse::<iroh_blobs::Hash>().is_ok()
}

fn sanitize_incoming_name(name: Option<&str>) -> Option<String> {
    name.and_then(|n| crate::storage::path_utils::sanitize_virtual_path(n).ok())
}

/// Catalog synchronization messages exchanged between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Full catalog sync containing all workspace records known to the node.
    CatalogDump(Vec<BlobRecord>),
    /// Notification that a new file was uploaded to the workspace.
    FileAdded(BlobRecord),
    /// Notification that a file was removed from the workspace by its authorized owner.
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
    /// A peer voted to KEEP / DEFEND a file against deletion.
    FileVoteKeep {
        hash: String,
        voter_node_id: String,
    },
    /// A peer retracted their vote to keep a file.
    FileUnvoteKeep {
        hash: String,
        voter_node_id: String,
    },
    /// A file's path/name was moved or updated.
    FileMoved {
        hash: String,
        new_name: String,
    },
    /// A node started hosting / seeding a blob (downloaded or uploaded).
    FileSeeded {
        hash: String,
        seeder_node_id: String,
    },
    /// A node stopped hosting a blob (e.g. evicted local cache).
    FileUnseeded {
        hash: String,
        seeder_node_id: String,
    },
}

/// Protocol handler registered on the Iroh Router for incoming catalog sync streams.
#[derive(Debug, Clone)]
pub struct CatalogSyncHandler {
    db: Arc<Database>,
    store: Option<iroh_blobs::store::fs::Store>,
}

impl CatalogSyncHandler {
    pub fn new(db: Arc<Database>, store: Option<iroh_blobs::store::fs::Store>) -> Self {
        Self { db, store }
    }

    /// Process an incoming message and update local SQLite catalog.
    pub async fn handle_message(&self, msg: SyncMessage, sender_peer_id: Option<&str>) -> Result<Option<SyncMessage>> {
        match msg {
            SyncMessage::CatalogDump(remote_records) => {
                for r in remote_records {
                    if !is_valid_blob_hash(&r.hash) {
                        continue;
                    }
                    let clean_name = sanitize_incoming_name(r.name.as_deref());
                    let _ = upsert_blob(
                        &self.db,
                        &r.hash,
                        clean_name.as_deref(),
                        r.size_bytes.min(100 * 1024 * 1024 * 1024),
                        false, // peer metadata starts unpinned locally until user pins or downloads
                        r.owner_node_id.as_deref(),
                    );
                    for voter in r.delete_votes {
                        let _ = cast_delete_vote(&self.db, &r.hash, &voter);
                    }
                    for voter in r.keep_votes {
                        let _ = cast_keep_vote(&self.db, &r.hash, &voter);
                    }
                    if r.is_cached {
                        if let Some(peer_id) = sender_peer_id {
                            let _ = record_peer_seed(&self.db, &r.hash, peer_id);
                        }
                    } else if let Some(peer_id) = sender_peer_id {
                        let _ = remove_peer_seed(&self.db, &r.hash, peer_id);
                    }
                }
                // Respond with our own catalog dump with accurate is_cached flags
                let mut my_records = list_all_blobs(&self.db).unwrap_or_default();
                if let Some(ref s) = self.store {
                    for rec in &mut my_records {
                        if let Ok(parsed) = rec.hash.parse::<iroh_blobs::Hash>() {
                            if let Ok(Some(entry)) = s.get(&parsed).await {
                                rec.is_cached = entry.is_complete();
                            }
                        }
                    }
                }
                Ok(Some(SyncMessage::CatalogDump(my_records)))
            }
            SyncMessage::FileAdded(r) => {
                if !is_valid_blob_hash(&r.hash) {
                    return Ok(None);
                }
                let clean_name = sanitize_incoming_name(r.name.as_deref());
                let _ = upsert_blob(
                    &self.db,
                    &r.hash,
                    clean_name.as_deref(),
                    r.size_bytes.min(100 * 1024 * 1024 * 1024),
                    false,
                    r.owner_node_id.as_deref(),
                );
                for voter in r.delete_votes {
                    let _ = cast_delete_vote(&self.db, &r.hash, &voter);
                }
                for voter in r.keep_votes {
                    let _ = cast_keep_vote(&self.db, &r.hash, &voter);
                }
                if let Some(peer_id) = sender_peer_id {
                    let _ = record_peer_seed(&self.db, &r.hash, peer_id);
                }
                if let Some(owner) = &r.owner_node_id {
                    let _ = record_peer_seed(&self.db, &r.hash, owner);
                }
                Ok(None)
            }
            SyncMessage::FileDeleted { hash, deleter_node_id } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                // SECURITY: ONLY allow deletion if the deleter is the legitimate owner of the file!
                // Non-owners CANNOT unilaterally delete files across peers; they must cast delete votes.
                if let Ok(Some(blob)) = get_blob(&self.db, &hash) {
                    let is_owner = blob.owner_node_id.as_deref() == Some(&deleter_node_id);
                    let sender_matches = sender_peer_id.is_none() || sender_peer_id == Some(&deleter_node_id);
                    if is_owner && sender_matches {
                        let _ = delete_blob(&self.db, &hash);
                    } else {
                        tracing::warn!(
                            hash = %hash,
                            deleter = %deleter_node_id,
                            sender = ?sender_peer_id,
                            "Rejected unauthorized unilateral FileDeleted message from non-owner"
                        );
                    }
                }
                Ok(None)
            }
            SyncMessage::FileVoteDelete {
                hash,
                voter_node_id,
            } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                // SECURITY: Verify voter is the authenticated QUIC sender to prevent vote spoofing
                if sender_peer_id == Some(voter_node_id.as_str()) {
                    let _ = cast_delete_vote(&self.db, &hash, &voter_node_id);
                } else {
                    tracing::warn!(
                        hash = %hash,
                        voter = %voter_node_id,
                        sender = ?sender_peer_id,
                        "Rejected spoofed FileVoteDelete message"
                    );
                }
                Ok(None)
            }
            SyncMessage::FileUnvoteDelete {
                hash,
                voter_node_id,
            } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                if sender_peer_id == Some(voter_node_id.as_str()) {
                    let _ = remove_delete_vote(&self.db, &hash, &voter_node_id);
                } else {
                    tracing::warn!(
                        hash = %hash,
                        voter = %voter_node_id,
                        sender = ?sender_peer_id,
                        "Rejected spoofed FileUnvoteDelete message"
                    );
                }
                Ok(None)
            }
            SyncMessage::FileVoteKeep {
                hash,
                voter_node_id,
            } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                if sender_peer_id == Some(voter_node_id.as_str()) {
                    let _ = cast_keep_vote(&self.db, &hash, &voter_node_id);
                } else {
                    tracing::warn!(
                        hash = %hash,
                        voter = %voter_node_id,
                        sender = ?sender_peer_id,
                        "Rejected spoofed FileVoteKeep message"
                    );
                }
                Ok(None)
            }
            SyncMessage::FileUnvoteKeep {
                hash,
                voter_node_id,
            } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                if sender_peer_id == Some(voter_node_id.as_str()) {
                    let _ = remove_keep_vote(&self.db, &hash, &voter_node_id);
                } else {
                    tracing::warn!(
                        hash = %hash,
                        voter = %voter_node_id,
                        sender = ?sender_peer_id,
                        "Rejected spoofed FileUnvoteKeep message"
                    );
                }
                Ok(None)
            }
            SyncMessage::FileMoved { hash, new_name } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                // SECURITY: Only the file's owner can move or rename it, and target path must be sanitized
                if let Ok(Some(blob)) = get_blob(&self.db, &hash) {
                    let is_owner = blob.owner_node_id.as_deref() == sender_peer_id;
                    if is_owner {
                        if let Ok(clean_path) = crate::storage::path_utils::sanitize_virtual_path(&new_name) {
                            let _ = rename_blob(&self.db, &hash, &clean_path);
                        } else {
                            tracing::warn!(hash = %hash, new_name = %new_name, "Rejected FileMoved with invalid path");
                        }
                    } else {
                        tracing::warn!(
                            hash = %hash,
                            sender = ?sender_peer_id,
                            owner = ?blob.owner_node_id,
                            "Rejected unauthorized FileMoved message from non-owner"
                        );
                    }
                }
                Ok(None)
            }
            SyncMessage::FileSeeded { hash, seeder_node_id } => {
                let _ = record_peer_seed(&self.db, &hash, &seeder_node_id);
                Ok(None)
            }
            SyncMessage::FileUnseeded { hash, seeder_node_id } => {
                let _ = remove_peer_seed(&self.db, &hash, &seeder_node_id);
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
                if let Ok(Some(reply)) = this.handle_message(msg, sender_peer.as_deref()).await {
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
    store: Option<&iroh_blobs::store::fs::Store>,
    peer_addr: NodeAddr,
) -> Result<()> {
    let alpn = get_sync_alpn(db);
    let connection = endpoint.connect(peer_addr.clone(), &alpn).await?;
    let (mut send, mut recv) = connection.open_bi().await?;

    let mut my_records = list_all_blobs(db)?;
    if let Some(s) = store {
        for rec in &mut my_records {
            if let Ok(parsed) = rec.hash.parse::<iroh_blobs::Hash>() {
                if let Ok(Some(entry)) = s.get(&parsed).await {
                    rec.is_cached = entry.is_complete();
                }
            }
        }
    }
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
                if !is_valid_blob_hash(&r.hash) {
                    continue;
                }
                let clean_name = sanitize_incoming_name(r.name.as_deref());
                let _ = upsert_blob(
                    db,
                    &r.hash,
                    clean_name.as_deref(),
                    r.size_bytes.min(100 * 1024 * 1024 * 1024),
                    false,
                    r.owner_node_id.as_deref(),
                );
                for voter in r.delete_votes {
                    let _ = cast_delete_vote(db, &r.hash, &voter);
                }
                for voter in r.keep_votes {
                    let _ = cast_keep_vote(db, &r.hash, &voter);
                }
                if r.is_cached {
                    let _ = record_peer_seed(db, &r.hash, &peer_id_str);
                } else {
                    let _ = remove_peer_seed(db, &r.hash, &peer_id_str);
                }
            }
        }
    }

    Ok(())
}

/// Broadcast a catalog mutation event to all actively connected swarm peers.
pub async fn broadcast_catalog_event(endpoint: &Endpoint, alpn: &[u8], msg: SyncMessage) {
    let bytes = match serde_json::to_vec(&msg) {
        Ok(b) => b,
        Err(_) => return,
    };

    let alpn_vec = alpn.to_vec();
    let remotes: Vec<_> = endpoint.remote_info_iter().collect();
    for remote in remotes {
        let peer_addr = NodeAddr::new(remote.node_id);
        let ep = endpoint.clone();
        let payload = bytes.clone();
        let proto = alpn_vec.clone();
        tokio::spawn(async move {
            if let Ok(conn) = ep.connect(peer_addr, &proto).await {
                if let Ok((mut send, _)) = conn.open_bi().await {
                    let _ = send.write_all(&payload).await;
                    let _ = send.finish();
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sync_alpn_derivation() {
        let dir = TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();

        // Default when secret is unset
        let default_alpn = get_sync_alpn(&db);
        assert_eq!(default_alpn, b"/p2pstud/sync/1");

        // When secret is configured
        db.set_setting("network_secret", "course-math-2026").unwrap();
        let math_alpn = get_sync_alpn(&db);
        assert!(math_alpn.starts_with(b"/p2pstud/sync/1/"));
        assert_ne!(math_alpn, default_alpn);

        // Different secret produces different ALPN
        db.set_setting("network_secret", "course-cs-2026").unwrap();
        let cs_alpn = get_sync_alpn(&db);
        assert_ne!(math_alpn, cs_alpn);

        // Whitespace is trimmed
        db.set_setting("network_secret", "  course-math-2026  ").unwrap();
        assert_eq!(get_sync_alpn(&db), math_alpn);
    }

    #[test]
    fn test_blob_hash_validation() {
        assert!(is_valid_blob_hash("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));
        assert!(!is_valid_blob_hash("invalid"));
        assert!(!is_valid_blob_hash(""));
        assert!(!is_valid_blob_hash("../../../etc/passwd"));
    }
}
