// src-tauri/src/p2p/sync.rs
// Workspace catalog synchronization protocol across swarm peers over QUIC.
// ALPN: /p2pstud/sync/1

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use iroh::{Endpoint, EndpointAddr, PublicKey, SecretKey, Signature};
use iroh::protocol::ProtocolHandler;
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
    /// A peer rated a file (+1, 0, or -1).
    FileRated {
        hash: String,
        voter_node_id: String,
        rating: i8,
    },
    /// A peer reported a file as dangerous.
    FileQuarantineReport {
        hash: String,
        reporter_node_id: String,
    },
    /// Bilateral cryptographic receipt proving chunks were served by a seeder to a recipient.
    SeedingReceipt {
        hash: String,
        seeder_node_id: String,
        recipient_node_id: String,
        bytes_transferred: u64,
        issued_at: i64,
        signature_hex: String,
    },
    /// A virtual folder was created in the workspace.
    FolderCreated {
        path: String,
    },
    /// An empty virtual folder was deleted from the workspace.
    FolderDeleted {
        path: String,
    },
}

/// Helper function to generate conflict-resolved filename preserving extension.
pub fn resolve_path_conflict(original_path: &str, short_id: &str) -> String {
    if let Some((base, ext)) = original_path.rsplit_once('.') {
        format!("{base} ({short_id}).{ext}")
    } else {
        format!("{original_path} ({short_id})")
    }
}

pub fn create_receipt_payload(
    hash: &str,
    seeder: &str,
    recipient: &str,
    bytes: u64,
    issued_at: i64,
) -> String {
    format!("receipt:{hash}:{seeder}:{recipient}:{bytes}:{issued_at}")
}

pub fn sign_receipt(secret_key: &SecretKey, payload: &str) -> String {
    let sig = secret_key.sign(payload.as_bytes());
    let bytes = sig.to_bytes();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn verify_receipt_signature(
    recipient_node_id: &str,
    payload: &str,
    signature_hex: &str,
) -> bool {
    let Ok(pub_key) = recipient_node_id.parse::<PublicKey>() else {
        return false;
    };
    if signature_hex.len() != 128 {
        return false;
    }
    let mut sig_bytes = [0u8; 64];
    for i in 0..64 {
        sig_bytes[i] = match u8::from_str_radix(&signature_hex[i * 2..i * 2 + 2], 16) {
            Ok(b) => b,
            Err(_) => return false,
        };
    }
    let sig = Signature::from_bytes(&sig_bytes);
    pub_key.verify(payload.as_bytes(), &sig).is_ok()
}

/// Protocol handler registered on the Iroh Router for incoming catalog sync streams.
#[derive(Debug, Clone)]
pub struct CatalogSyncHandler {
    db: Arc<Database>,
    store: Option<iroh_blobs::store::fs::FsStore>,
}

impl CatalogSyncHandler {
    pub fn new(db: Arc<Database>, store: Option<iroh_blobs::store::fs::FsStore>) -> Self {
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
                            rec.is_cached = matches!(
                                s.blobs().status(parsed).await,
                                Ok(iroh_blobs::api::blobs::BlobStatus::Complete { .. })
                            );
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

                if let Some(target_name) = clean_name.as_deref() {
                    let existing: Option<(String, i64)> = {
                        let conn = self.db.lock();
                        use rusqlite::OptionalExtension;
                        conn.query_row(
                            "SELECT hash, last_accessed_at FROM local_blobs WHERE name = ?1 LIMIT 1",
                            rusqlite::params![target_name],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .optional()
                        .unwrap_or(None)
                    };

                    if let Some((local_hash, local_time)) = existing {
                        if local_hash != r.hash {
                            // Offline split-brain conflict resolution (Step 37)
                            let incoming_time = r.catalog_first_seen_at.unwrap_or(r.last_accessed_at);
                            let incoming_wins = if incoming_time != local_time {
                                incoming_time < local_time
                            } else {
                                r.hash < local_hash
                            };

                            let sender_short = sender_peer_id
                                .map(|s| &s[..8.min(s.len())])
                                .unwrap_or("peer");
                            let local_short = &local_hash[..8.min(local_hash.len())];

                            if incoming_wins {
                                // Incoming file is older -> it wins original path
                                let new_local_name = resolve_path_conflict(target_name, local_short);
                                let _ = rename_blob(&self.db, &local_hash, &new_local_name);
                                let _ = upsert_blob(
                                    &self.db,
                                    &r.hash,
                                    Some(target_name),
                                    r.size_bytes.min(100 * 1024 * 1024 * 1024),
                                    false,
                                    r.owner_node_id.as_deref(),
                                );
                            } else {
                                // Local file is older -> it wins original path
                                let new_incoming_name = resolve_path_conflict(target_name, sender_short);
                                let _ = upsert_blob(
                                    &self.db,
                                    &r.hash,
                                    Some(&new_incoming_name),
                                    r.size_bytes.min(100 * 1024 * 1024 * 1024),
                                    false,
                                    r.owner_node_id.as_deref(),
                                );
                            }
                        } else {
                            let _ = upsert_blob(
                                &self.db,
                                &r.hash,
                                Some(target_name),
                                r.size_bytes.min(100 * 1024 * 1024 * 1024),
                                false,
                                r.owner_node_id.as_deref(),
                            );
                        }
                    } else {
                        let _ = upsert_blob(
                            &self.db,
                            &r.hash,
                            Some(target_name),
                            r.size_bytes.min(100 * 1024 * 1024 * 1024),
                            false,
                            r.owner_node_id.as_deref(),
                        );
                    }
                }

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
            SyncMessage::FileRated {
                hash,
                voter_node_id,
                rating,
            } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                if sender_peer_id == Some(voter_node_id.as_str()) {
                    let _ = crate::storage::blob_tracker::rate_file(&self.db, &hash, &voter_node_id, rating);
                }
                Ok(None)
            }
            SyncMessage::FileQuarantineReport {
                hash,
                reporter_node_id,
            } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                if sender_peer_id == Some(reporter_node_id.as_str()) {
                    let _ = crate::storage::blob_tracker::report_dangerous(&self.db, &hash, &reporter_node_id);
                }
                Ok(None)
            }
            SyncMessage::SeedingReceipt {
                hash,
                seeder_node_id,
                recipient_node_id,
                bytes_transferred,
                issued_at,
                signature_hex,
            } => {
                if !is_valid_blob_hash(&hash) {
                    return Ok(None);
                }
                // 1. Self-dealing check: reject if recipient is seeder
                if recipient_node_id == seeder_node_id {
                    tracing::warn!("Discarded self-dealing seeding receipt");
                    return Ok(None);
                }

                // 2. Must exist in workspace catalog
                if get_blob(&self.db, &hash).unwrap_or(None).is_none() {
                    return Ok(None);
                }

                // 3. Cryptographic signature check
                let payload = create_receipt_payload(&hash, &seeder_node_id, &recipient_node_id, bytes_transferred, issued_at);
                if verify_receipt_signature(&recipient_node_id, &payload, &signature_hex) {
                    let _ = crate::storage::blob_tracker::store_seeding_receipt(
                        &self.db,
                        &hash,
                        &seeder_node_id,
                        &recipient_node_id,
                        bytes_transferred,
                        issued_at,
                        &signature_hex,
                    );
                    tracing::info!(seeder = %seeder_node_id, recipient = %recipient_node_id, "Verified and stored bilateral seeding receipt");
                } else {
                    tracing::warn!("Discarded seeding receipt with invalid signature");
                }

                // TODO(governance): When receipt-based governance is enforced, nodes with 0 verified receipts
                // within 14 days will have no vote weight. For now, log this count alongside votes but do NOT
                // reject votes based on receipt count.

                Ok(None)
            }
            SyncMessage::FolderCreated { path } => {
                let _ = crate::storage::blob_tracker::create_virtual_folder(&self.db, &path);
                Ok(None)
            }
            SyncMessage::FolderDeleted { path } => {
                let _ = crate::storage::blob_tracker::delete_virtual_folder(&self.db, &path);
                Ok(None)
            }
        }
    }
}

impl ProtocolHandler for CatalogSyncHandler {
    fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> impl Future<Output = std::result::Result<(), iroh::protocol::AcceptError>> + Send {
        let this = self.clone();
        async move {
            let remote_id = connection.remote_id();
            register_active_peer(remote_id);
            let sender_peer = Some(remote_id.to_string());
            let (mut send, mut recv) = connection.accept_bi().await?;

            // Read incoming message with 2MB limit (plenty for thousands of catalog entries)
            let bytes = recv
                .read_to_end(2 * 1024 * 1024)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            if let Ok(msg) = serde_json::from_slice::<SyncMessage>(&bytes) {
                if let Ok(Some(reply)) = this.handle_message(msg, sender_peer.as_deref()).await {
                    if let Ok(reply_bytes) = serde_json::to_vec(&reply) {
                        send.write_all(&reply_bytes)
                            .await
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                    }
                }
            }
            let _ = send.finish();
            Ok(())
        }
    }
}

static ACTIVE_SWARM_PEERS: std::sync::RwLock<Vec<iroh::EndpointId>> = std::sync::RwLock::new(Vec::new());

pub fn register_active_peer(peer_id: iroh::EndpointId) {
    if let Ok(mut lock) = ACTIVE_SWARM_PEERS.write() {
        if !lock.contains(&peer_id) {
            lock.push(peer_id);
        }
    }
}

/// Perform a bi-directional catalog exchange with a remote peer.
pub async fn sync_catalog_with_peer(
    endpoint: &Endpoint,
    db: &Arc<Database>,
    store: Option<&iroh_blobs::store::fs::FsStore>,
    peer_addr: EndpointAddr,
) -> Result<()> {
    register_active_peer(peer_addr.id);
    let alpn = get_sync_alpn(db);
    let connection = endpoint.connect(peer_addr.clone(), &alpn).await?;
    let (mut send, mut recv) = connection.open_bi().await?;

    let mut my_records = list_all_blobs(db)?;
    if let Some(s) = store {
        for rec in &mut my_records {
            if let Ok(parsed) = rec.hash.parse::<iroh_blobs::Hash>() {
                if let Ok(status) = s.blobs().status(parsed).await {
                    rec.is_cached = matches!(status, iroh_blobs::api::blobs::BlobStatus::Complete { .. });
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
            let peer_id_str = peer_addr.id.to_string();
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
    let peers: Vec<iroh::EndpointId> = {
        if let Ok(lock) = ACTIVE_SWARM_PEERS.read() {
            lock.clone()
        } else {
            Vec::new()
        }
    };

    for peer_id in peers {
        let peer_addr = EndpointAddr::from(peer_id);
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

    #[test]
    fn test_seeding_receipt_signing_and_verification() {
        let recipient_key = SecretKey::generate();
        let recipient_node_id = recipient_key.public().to_string();
        let seeder_key = SecretKey::generate();
        let seeder_node_id = seeder_key.public().to_string();

        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let payload = create_receipt_payload(hash, &seeder_node_id, &recipient_node_id, 1048576, 1700000000);

        let signature_hex = sign_receipt(&recipient_key, &payload);
        assert_eq!(signature_hex.len(), 128);

        // Verification with correct key succeeds
        assert!(verify_receipt_signature(&recipient_node_id, &payload, &signature_hex));

        // Tampered payload fails
        let tampered_payload = create_receipt_payload(hash, &seeder_node_id, &recipient_node_id, 2048576, 1700000000);
        assert!(!verify_receipt_signature(&recipient_node_id, &tampered_payload, &signature_hex));

        // Wrong signer key fails
        assert!(!verify_receipt_signature(&seeder_node_id, &payload, &signature_hex));
    }

    #[tokio::test]
    async fn test_offline_conflict_resolution_older_wins() {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(&dir.path().join("test.db")).unwrap());
        let handler = CatalogSyncHandler::new(db.clone(), None);

        let local_hash = "0000000000000000000000000000000000000000000000000000000000000001";
        let incoming_hash = "0000000000000000000000000000000000000000000000000000000000000002";

        // Local file created at timestamp 2000
        upsert_blob(&db, local_hash, Some("notes.pdf"), 500, false, Some("nodeA")).unwrap();
        {
            let conn = db.lock();
            conn.execute(
                "UPDATE local_blobs SET last_accessed_at = 2000, catalog_first_seen_at = 2000 WHERE hash = ?1",
                rusqlite::params![local_hash],
            ).unwrap();
        }

        // Incoming file created earlier at timestamp 1000 with same name "notes.pdf"
        let incoming_record = BlobRecord {
            hash: incoming_hash.to_string(),
            name: Some("notes.pdf".to_string()),
            size_bytes: 600,
            last_accessed_at: 1000,
            is_pinned: false,
            owner_node_id: Some("nodeB".to_string()),
            catalog_first_seen_at: Some(1000),
            ..Default::default()
        };

        let _ = handler.handle_message(SyncMessage::FileAdded(incoming_record), Some("nodeB_peer_id")).await;

        // Incoming file wins original name "notes.pdf"
        let incoming_blob = get_blob(&db, incoming_hash).unwrap().unwrap();
        assert_eq!(incoming_blob.name.as_deref(), Some("notes.pdf"));

        // Local file was deterministically renamed with its short ID
        let local_blob = get_blob(&db, local_hash).unwrap().unwrap();
        let local_name = local_blob.name.unwrap();
        assert!(local_name.starts_with("notes (") && local_name.ends_with(").pdf"));
    }
}
