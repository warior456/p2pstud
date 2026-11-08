// src-tauri/src/storage/blob_tracker.rs
// CRUD operations on the local_blobs SQLite table and file_delete_votes table.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::storage::db::Database;

/// Number of community delete votes required to trigger deletion grace period.
pub const DELETE_VOTE_QUORUM: usize = 3;

/// Required delete vote ratio (86% of total cast votes: D / (D + K) >= 0.86).
pub const DELETE_VOTE_RATIO_THRESHOLD: f64 = 0.86;

/// Grace period in seconds before community deletion is finalized (24 hours).
pub const DELETION_GRACE_PERIOD_SECS: i64 = 86400;

/// Status resulting from evaluating a blob's deletion votes against quorum and ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeletionScheduleStatus {
    Safe,
    Scheduled(i64),
    ExpiredAndDeleted,
}

/// A cryptographically signed deletion tombstone proving that the genuine owner authorized file deletion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletionTombstone {
    pub hash: String,
    pub owner_node_id: String,
    pub deleted_at: i64,
    pub signature: String,
}

impl DeletionTombstone {
    /// Create and cryptographically sign a deletion tombstone using the owner's SecretKey.
    pub fn create_signed(
        hash: String,
        owner_node_id: String,
        deleted_at: i64,
        secret_key: &iroh::SecretKey,
    ) -> Self {
        let payload = format!("DELETE:{}:{}", hash, deleted_at);
        let sig = secret_key.sign(payload.as_bytes());
        let signature = hex::encode(sig.to_bytes());
        Self {
            hash,
            owner_node_id,
            deleted_at,
            signature,
        }
    }

    /// Verify that the tombstone signature is cryptographically valid and signed by owner_node_id.
    pub fn verify(&self) -> bool {
        let Ok(pk) = self.owner_node_id.parse::<iroh::PublicKey>() else {
            return false;
        };
        let payload = format!("DELETE:{}:{}", self.hash, self.deleted_at);
        let Ok(sig_bytes) = hex::decode(&self.signature) else {
            return false;
        };
        if sig_bytes.len() != 64 {
            return false;
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&sig_bytes);
        let sig = iroh::Signature::from_bytes(&arr);
        pk.verify(payload.as_bytes(), &sig).is_ok()
    }
}

/// Check if a blob has an active deletion tombstone.
pub fn is_blob_tombstoned(db: &Database, hash: &str) -> Result<bool> {
    let conn = db.lock();
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM deleted_blobs WHERE hash = ?1 LIMIT 1",
            params![hash],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    Ok(exists)
}

/// Retrieve all recorded deletion tombstones.
pub fn list_all_tombstones(db: &Database) -> Result<Vec<DeletionTombstone>> {
    let conn = db.lock();
    let mut stmt = conn.prepare(
        "SELECT hash, owner_node_id, deleted_at, signature FROM deleted_blobs ORDER BY deleted_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(DeletionTombstone {
            hash: r.get(0)?,
            owner_node_id: r.get(1)?,
            deleted_at: r.get(2)?,
            signature: r.get(3)?,
        })
    })?;
    let mut tombstones = Vec::new();
    for row in rows {
        tombstones.push(row?);
    }
    Ok(tombstones)
}

/// Remove a tombstone (e.g. if the owner deliberately re-uploads the file).
pub fn remove_deletion_tombstone(db: &Database, hash: &str) -> Result<()> {
    let conn = db.lock();
    conn.execute("DELETE FROM deleted_blobs WHERE hash = ?1", params![hash])?;
    Ok(())
}

/// Validate and apply an incoming deletion tombstone.
/// Ensures the cryptographic signature verifies and that the signer matches the known owner.
pub fn record_deletion_tombstone(db: &Database, tombstone: &DeletionTombstone) -> Result<bool> {
    record_deletion_tombstone_ext(db, tombstone, false)
}

/// Record a locally initiated deletion tombstone. Always persists locally to prevent
/// peer sync resurrection even if the local node was not the original uploader.
pub fn record_deletion_tombstone_local(db: &Database, tombstone: &DeletionTombstone) -> Result<bool> {
    record_deletion_tombstone_ext(db, tombstone, true)
}

pub fn record_deletion_tombstone_ext(db: &Database, tombstone: &DeletionTombstone, is_local_origin: bool) -> Result<bool> {
    // 1. Verify cryptographic Ed25519 signature
    if !tombstone.verify() {
        tracing::warn!(
            hash = %tombstone.hash,
            owner = %tombstone.owner_node_id,
            "Rejected deletion tombstone with invalid cryptographic signature"
        );
        return Ok(false);
    }

    // 2. If the file is known locally and this is a remote tombstone from the network,
    // verify that the tombstone owner matches the known owner to prevent deletion spoofing.
    if !is_local_origin {
        if let Ok(Some(existing)) = get_blob(db, &tombstone.hash) {
            if let Some(ref known_owner) = existing.owner_node_id {
                if !crate::storage::db::is_matching_node_id(known_owner, &tombstone.owner_node_id) {
                    tracing::warn!(
                        hash = %tombstone.hash,
                        known_owner = %known_owner,
                        signer = %tombstone.owner_node_id,
                        "Rejected deletion tombstone: signer is not the registered owner of this file"
                    );
                    return Ok(false);
                }
            }
        }
    }

    // 3. Persist verified tombstone in SQLite
    {
        let conn = db.lock();
        conn.execute(
            "INSERT INTO deleted_blobs(hash, owner_node_id, deleted_at, signature)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(hash) DO UPDATE SET
                 owner_node_id = excluded.owner_node_id,
                 deleted_at    = MAX(deleted_blobs.deleted_at, excluded.deleted_at),
                 signature     = excluded.signature",
            params![
                tombstone.hash,
                tombstone.owner_node_id,
                tombstone.deleted_at,
                tombstone.signature,
            ],
        )?;
    }

    // 4. Remove blob metadata from local_blobs tracker
    delete_blob(db, &tombstone.hash)?;

    // 5. Physically delete cached files from disk
    let blob_dir = db.resolve_blob_dir();
    crate::storage::path_utils::delete_blob_files_from_disk(&blob_dir, &tombstone.hash);

    tracing::info!(hash = %tombstone.hash, owner = %tombstone.owner_node_id, "Applied verified deletion tombstone");
    Ok(true)
}

/// A row from the local_blobs table with associated community delete votes, keep votes, seed count, ratings, and quarantine status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobRecord {
    pub hash: String,
    pub name: Option<String>,
    pub size_bytes: u64,
    pub last_accessed_at: i64,
    pub is_pinned: bool,
    pub owner_node_id: Option<String>,
    #[serde(default)]
    pub delete_votes: Vec<String>,
    #[serde(default)]
    pub keep_votes: Vec<String>,
    #[serde(default)]
    pub deletion_scheduled_at: Option<i64>,
    #[serde(default = "default_seed_count")]
    pub seed_count: usize,
    #[serde(default)]
    pub is_cached: bool,
    #[serde(default)]
    pub written_at: Option<i64>,
    #[serde(default)]
    pub is_quarantined: bool,
    #[serde(default)]
    pub quarantine_expires_at: Option<i64>,
    #[serde(default)]
    pub upvotes: usize,
    #[serde(default)]
    pub downvotes: usize,
    #[serde(default)]
    pub net_rating: i64,
    #[serde(default)]
    pub is_starred: bool,
    #[serde(default)]
    pub catalog_first_seen_at: Option<i64>,
    #[serde(default)]
    pub last_any_seed_seen_at: Option<i64>,
}

impl Default for BlobRecord {
    fn default() -> Self {
        let now = now_unix();
        Self {
            hash: String::new(),
            name: None,
            size_bytes: 0,
            last_accessed_at: now,
            is_pinned: false,
            owner_node_id: None,
            delete_votes: Vec::new(),
            keep_votes: Vec::new(),
            deletion_scheduled_at: None,
            seed_count: 1,
            is_cached: false,
            written_at: Some(now),
            is_quarantined: false,
            quarantine_expires_at: None,
            upvotes: 0,
            downvotes: 0,
            net_rating: 0,
            is_starred: false,
            catalog_first_seen_at: Some(now),
            last_any_seed_seen_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineStatus {
    pub is_quarantined: bool,
    pub report_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferQueueItem {
    pub hash: String,
    pub name: Option<String>,
    pub total_bytes: u64,
    pub received_bytes: u64,
    pub source_peer: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoleSeedFile {
    pub hash: String,
    pub name: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedingReceipt {
    pub hash: String,
    pub seeder_node_id: String,
    pub recipient_node_id: String,
    pub bytes_transferred: u64,
    pub issued_at: i64,
    pub signature: String,
}

fn default_seed_count() -> usize {
    1
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX_EPOCH")
        .as_secs() as i64
}

/// Retrieve all voter Node IDs who marked this file for deletion.
pub fn get_delete_votes(db: &Database, hash: &str) -> Result<Vec<String>> {
    let conn = db.lock();
    let mut stmt = conn.prepare("SELECT voter_node_id FROM file_delete_votes WHERE hash = ?1")?;
    let rows = stmt.query_map(params![hash], |r| r.get(0))?;
    let mut votes = Vec::new();
    for v in rows {
        votes.push(v?);
    }
    Ok(votes)
}

/// Retrieve all voter Node IDs who voted to KEEP/DEFEND this file against deletion.
pub fn get_keep_votes(db: &Database, hash: &str) -> Result<Vec<String>> {
    let conn = db.lock();
    let mut stmt = conn.prepare("SELECT voter_node_id FROM file_keep_votes WHERE hash = ?1")?;
    let rows = stmt.query_map(params![hash], |r| r.get(0))?;
    let mut votes = Vec::new();
    for v in rows {
        votes.push(v?);
    }
    Ok(votes)
}

/// Evaluates community delete and keep votes for a blob.
/// If Delete Quorum >= 3 AND Delete / (Delete + Keep) >= 86%:
///   - Schedules deletion grace period (24 hours) if not yet scheduled.
///   - If grace period has elapsed, purges the blob and returns `ExpiredAndDeleted`.
///
/// If votes drop below 86% or quorum:
///   - Clears any scheduled deletion (cancels grace period) and returns `Safe`.
pub fn evaluate_deletion_schedule(db: &Database, hash: &str) -> Result<DeletionScheduleStatus> {
    let d_votes = get_delete_votes(db, hash)?.len();
    let k_votes = get_keep_votes(db, hash)?.len();
    let total_votes = d_votes + k_votes;

    let scheduled_at: Option<i64> = {
        let conn = db.lock();
        conn.query_row(
            "SELECT deletion_scheduled_at FROM local_blobs WHERE hash = ?1",
            params![hash],
            |r| r.get(0),
        )
        .unwrap_or(None)
    };

    let ratio = if total_votes > 0 {
        d_votes as f64 / total_votes as f64
    } else {
        0.0
    };

    let now = now_unix();
    if d_votes >= DELETE_VOTE_QUORUM && ratio >= DELETE_VOTE_RATIO_THRESHOLD {
        match scheduled_at {
            Some(sched) => {
                if now >= sched {
                    delete_blob(db, hash)?;
                    Ok(DeletionScheduleStatus::ExpiredAndDeleted)
                } else {
                    Ok(DeletionScheduleStatus::Scheduled(sched))
                }
            }
            None => {
                let sched = now + DELETION_GRACE_PERIOD_SECS;
                let conn = db.lock();
                let _ = conn.execute(
                    "UPDATE local_blobs SET deletion_scheduled_at = ?1 WHERE hash = ?2",
                    params![sched, hash],
                );
                Ok(DeletionScheduleStatus::Scheduled(sched))
            }
        }
    } else {
        if scheduled_at.is_some() {
            let conn = db.lock();
            let _ = conn.execute(
                "UPDATE local_blobs SET deletion_scheduled_at = NULL WHERE hash = ?1",
                params![hash],
            );
        }
        Ok(DeletionScheduleStatus::Safe)
    }
}

/// Cast a vote to delete a file. Automatically triggers schedule evaluation.
pub fn cast_delete_vote(db: &Database, hash: &str, voter_node_id: &str) -> Result<usize> {
    {
        let conn = db.lock();
        conn.execute(
            "INSERT OR IGNORE INTO file_delete_votes(hash, voter_node_id, voted_at) VALUES(?1, ?2, ?3)",
            params![hash, voter_node_id, now_unix()],
        )?;
    }
    let _ = evaluate_deletion_schedule(db, hash);
    let conn = db.lock();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_delete_votes WHERE hash = ?1",
        params![hash],
        |r| r.get(0),
    )?;
    Ok(count as usize)
}

/// Retract a delete vote. Automatically recalculates deletion schedule.
pub fn remove_delete_vote(db: &Database, hash: &str, voter_node_id: &str) -> Result<usize> {
    {
        let conn = db.lock();
        conn.execute(
            "DELETE FROM file_delete_votes WHERE hash = ?1 AND voter_node_id = ?2",
            params![hash, voter_node_id],
        )?;
    }
    let _ = evaluate_deletion_schedule(db, hash);
    let conn = db.lock();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_delete_votes WHERE hash = ?1",
        params![hash],
        |r| r.get(0),
    )?;
    Ok(count as usize)
}

/// Cast a vote to KEEP / DEFEND a file against deletion. Automatically triggers schedule evaluation.
pub fn cast_keep_vote(db: &Database, hash: &str, voter_node_id: &str) -> Result<usize> {
    {
        let conn = db.lock();
        conn.execute(
            "INSERT OR IGNORE INTO file_keep_votes(hash, voter_node_id, voted_at) VALUES(?1, ?2, ?3)",
            params![hash, voter_node_id, now_unix()],
        )?;
    }
    let _ = evaluate_deletion_schedule(db, hash);
    let conn = db.lock();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_keep_votes WHERE hash = ?1",
        params![hash],
        |r| r.get(0),
    )?;
    Ok(count as usize)
}

/// Retract a keep vote. Automatically recalculates deletion schedule.
pub fn remove_keep_vote(db: &Database, hash: &str, voter_node_id: &str) -> Result<usize> {
    {
        let conn = db.lock();
        conn.execute(
            "DELETE FROM file_keep_votes WHERE hash = ?1 AND voter_node_id = ?2",
            params![hash, voter_node_id],
        )?;
    }
    let _ = evaluate_deletion_schedule(db, hash);
    let conn = db.lock();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_keep_votes WHERE hash = ?1",
        params![hash],
        |r| r.get(0),
    )?;
    Ok(count as usize)
}

/// Record that a peer node holds / is actively seeding a blob.
pub fn record_peer_seed(db: &Database, hash: &str, peer_node_id: &str) -> Result<()> {
    let conn = db.lock();
    let last_seen: Option<i64> = conn
        .query_row(
            "SELECT last_seen_at FROM file_seeds WHERE hash = ?1 AND peer_node_id = ?2",
            params![hash, peer_node_id],
            |r| r.get(0),
        )
        .optional()?;

    let now = now_unix();
    if let Some(prev) = last_seen {
        if now - prev < 180 {
            // Already updated recently: avoid continuous WAL writes during sync cycles
            return Ok(());
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO file_seeds(hash, peer_node_id, last_seen_at) VALUES(?1, ?2, ?3)",
        params![hash, peer_node_id, now],
    )?;
    Ok(())
}

/// Remove a peer seed entry (e.g. if peer evicted or deleted the file).
pub fn remove_peer_seed(db: &Database, hash: &str, peer_node_id: &str) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "DELETE FROM file_seeds WHERE hash = ?1 AND peer_node_id = ?2",
        params![hash, peer_node_id],
    )?;
    Ok(())
}

/// Calculate active seed count for a blob across the swarm.
/// Peers seen within the last 30 minutes are considered active seeders.
/// Excludes local_node_id from peer seeds to avoid double-counting the local node.
pub fn get_seed_count(
    db: &Database,
    hash: &str,
    local_node_id: Option<&str>,
    is_locally_cached: bool,
) -> Result<usize> {
    let conn = db.lock();
    let threshold = now_unix() - 1800; // 30 minutes
    let mut stmt = conn.prepare(
        "SELECT DISTINCT peer_node_id FROM file_seeds WHERE hash = ?1 AND last_seen_at >= ?2",
    )?;
    let rows = stmt.query_map(params![hash, threshold], |r| r.get::<_, String>(0))?;
    let mut peer_seeds = 0;
    for row in rows.flatten() {
        if let Some(my_id) = local_node_id {
            if crate::storage::db::is_matching_node_id(&row, my_id) {
                continue;
            }
        }
        peer_seeds += 1;
    }

    let mut total = peer_seeds;
    if is_locally_cached {
        total += 1;
    }
    Ok(total)
}

/// Insert or replace a blob record (upsert).
/// Enforces path collision protection: duplicate path with a different hash returns an error.
pub fn upsert_blob(
    db: &Database,
    hash: &str,
    name: Option<&str>,
    size_bytes: u64,
    is_pinned: bool,
    owner_node_id: Option<&str>,
) -> Result<()> {
    // If this file has an active deletion tombstone, reject upsert so deleted files are never resurrected
    if is_blob_tombstoned(db, hash)? {
        tracing::debug!(hash = %hash, "Skipping upsert for tombstoned (deleted) blob");
        return Ok(());
    }

    let now = now_unix();
    let conn = db.lock();

    // Check if the virtual path is already taken by a different hash
    if let Some(target_name) = name {
        let existing_hash: Option<String> = conn
            .query_row(
                "SELECT hash FROM local_blobs WHERE name = ?1 LIMIT 1",
                params![target_name],
                |r| r.get(0),
            )
            .optional()?;

        if let Some(existing) = existing_hash {
            if existing != hash {
                anyhow::bail!("Path already occupied by a different file. Delete the existing file first.");
            }
        }
    }

    // Performance optimization: Check if blob already exists with identical attributes.
    // If unchanged, skip writing to disk to avoid continuous WAL churning during peer catalog sync.
    let existing_info: Option<(Option<String>, i64, i64, Option<String>)> = conn
        .query_row(
            "SELECT name, size_bytes, is_pinned, owner_node_id FROM local_blobs WHERE hash = ?1",
            params![hash],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;

    if let Some((existing_name, existing_size, existing_pinned, existing_owner)) = existing_info {
        let name_matches = match (name, existing_name.as_deref()) {
            (None, _) => true,
            (Some(n), Some(en)) => n == en,
            (Some(_), None) => false,
        };
        let owner_matches = match (owner_node_id, existing_owner.as_deref()) {
            (None, _) => true,
            (Some(o), Some(eo)) => crate::storage::db::is_matching_node_id(o, eo),
            (Some(_), None) => false,
        };
        let pinned_matches = !is_pinned || (existing_pinned != 0);
        let size_matches = existing_size == size_bytes as i64;

        if name_matches && owner_matches && pinned_matches && size_matches {
            // Already completely synchronized and identical. Skip SQLite write.
            return Ok(());
        }
    }

    conn.execute(
        "INSERT INTO local_blobs(hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, written_at, catalog_first_seen_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(hash) DO UPDATE SET
             name             = COALESCE(excluded.name, local_blobs.name),
             size_bytes       = excluded.size_bytes,
             last_accessed_at = excluded.last_accessed_at,
             owner_node_id    = COALESCE(local_blobs.owner_node_id, excluded.owner_node_id)",
        params![
            hash,
            name,
            size_bytes as i64,
            now,
            is_pinned as i64,
            owner_node_id,
            now,
            now,
        ],
    )?;
    Ok(())
}

/// Update last_accessed_at to now (called when a blob is read/opened).
/// Debounced so repeated chunk access within 60s does not cause constant WAL writes.
pub fn touch_blob(db: &Database, hash: &str) -> Result<()> {
    let conn = db.lock();
    let now = now_unix();
    conn.execute(
        "UPDATE local_blobs SET last_accessed_at = ?1 WHERE hash = ?2 AND (?1 - last_accessed_at > 60)",
        params![now, hash],
    )?;
    Ok(())
}

/// Rename or move a blob to a new workspace path.
pub fn rename_blob(db: &Database, hash: &str, new_name: &str) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "UPDATE local_blobs SET name = ?1 WHERE hash = ?2",
        params![new_name, hash],
    )?;
    Ok(())
}

/// Set or clear the pinned flag on a blob.
pub fn set_pinned(db: &Database, hash: &str, pinned: bool) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "UPDATE local_blobs SET is_pinned = ?1 WHERE hash = ?2",
        params![pinned as i64, hash],
    )?;
    Ok(())
}

/// Delete a blob record and all its associated delete votes, keep votes, and seed records.
pub fn delete_blob(db: &Database, hash: &str) -> Result<()> {
    let conn = db.lock();
    let _ = conn.execute("DELETE FROM file_delete_votes WHERE hash = ?1", params![hash]);
    let _ = conn.execute("DELETE FROM file_keep_votes WHERE hash = ?1", params![hash]);
    let _ = conn.execute("DELETE FROM file_seeds WHERE hash = ?1", params![hash]);
    conn.execute("DELETE FROM local_blobs WHERE hash = ?1", params![hash])?;
    Ok(())
}

/// Return the total bytes used by all unpinned blobs.
pub fn total_unpinned_bytes(db: &Database) -> Result<u64> {
    let conn = db.lock();
    let bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM local_blobs WHERE is_pinned = 0",
        [],
        |r| r.get(0),
    )?;
    Ok(bytes as u64)
}

/// Return the total bytes used by all blobs (pinned + unpinned).
pub fn total_bytes(db: &Database) -> Result<u64> {
    let conn = db.lock();
    let bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM local_blobs",
        [],
        |r| r.get(0),
    )?;
    Ok(bytes as u64)
}

/// Return total bytes used by pinned blobs only.
pub fn total_pinned_bytes(db: &Database) -> Result<u64> {
    let conn = db.lock();
    let bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM local_blobs WHERE is_pinned = 1",
        [],
        |r| r.get(0),
    )?;
    Ok(bytes as u64)
}

/// Return unpinned blobs ordered oldest-first (for eviction).
pub fn oldest_unpinned(db: &Database, limit: usize) -> Result<Vec<BlobRecord>> {
    let mut records = {
        let conn = db.lock();
        let mut stmt = conn.prepare(
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at,
                    written_at, is_quarantined, quarantine_expires_at, catalog_first_seen_at, last_any_seed_seen_at
             FROM local_blobs
             WHERE is_pinned = 0
             ORDER BY last_accessed_at ASC
             LIMIT ?1",
        )?;
        let raw_rows = stmt.query_map(params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)? as u64,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)? != 0,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<i64>>(8)?.unwrap_or(0) != 0,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, Option<i64>>(11)?,
            ))
        })?;

        let mut list = Vec::new();
        for r in raw_rows {
            let (hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at,
                 written_at, is_quarantined, quarantine_expires_at, catalog_first_seen_at, last_any_seed_seen_at) = r?;
            list.push(BlobRecord {
                hash,
                name,
                size_bytes,
                last_accessed_at,
                is_pinned,
                owner_node_id,
                delete_votes: Vec::new(),
                keep_votes: Vec::new(),
                deletion_scheduled_at,
                seed_count: 1,
                is_cached: false,
                written_at,
                is_quarantined,
                quarantine_expires_at,
                upvotes: 0,
                downvotes: 0,
                net_rating: 0,
                is_starred: false,
                catalog_first_seen_at,
                last_any_seed_seen_at,
            });
        }
        list
    };

    for r in &mut records {
        r.delete_votes = get_delete_votes(db, &r.hash).unwrap_or_default();
        r.keep_votes = get_keep_votes(db, &r.hash).unwrap_or_default();
        r.seed_count = get_seed_count(db, &r.hash, None, false).unwrap_or(1);
        let (up, down, net) = get_file_ratings(db, &r.hash).unwrap_or((0, 0, 0));
        r.upvotes = up;
        r.downvotes = down;
        r.net_rating = net;
        r.is_starred = is_starred(db, &r.hash).unwrap_or(false);
    }

    Ok(records)
}

/// Look up a single blob by hash.
pub fn get_blob(db: &Database, hash: &str) -> Result<Option<BlobRecord>> {
    let result = {
        let conn = db.lock();
        conn.query_row(
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at,
                    written_at, is_quarantined, quarantine_expires_at, catalog_first_seen_at, last_any_seed_seen_at
             FROM local_blobs WHERE hash = ?1",
            params![hash],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)? as u64,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)? != 0,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<i64>>(8)?.unwrap_or(0) != 0,
                    row.get::<_, Option<i64>>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                ))
            },
        )
    };

    let (hash_val, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at,
         written_at, is_quarantined, quarantine_expires_at, catalog_first_seen_at, last_any_seed_seen_at) = match result {
        Ok(vals) => vals,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let delete_votes = get_delete_votes(db, &hash_val).unwrap_or_default();
    let keep_votes = get_keep_votes(db, &hash_val).unwrap_or_default();
    let seed_count = get_seed_count(db, &hash_val, None, false).unwrap_or(1);
    let (upvotes, downvotes, net_rating) = get_file_ratings(db, &hash_val).unwrap_or((0, 0, 0));
    let is_starred_val = is_starred(db, &hash_val).unwrap_or(false);

    Ok(Some(BlobRecord {
        hash: hash_val,
        name,
        size_bytes,
        last_accessed_at,
        is_pinned,
        owner_node_id,
        delete_votes,
        keep_votes,
        deletion_scheduled_at,
        seed_count,
        is_cached: false,
        written_at,
        is_quarantined,
        quarantine_expires_at,
        upvotes,
        downvotes,
        net_rating,
        is_starred: is_starred_val,
        catalog_first_seen_at,
        last_any_seed_seen_at,
    }))
}

/// Return all blobs in the database ordered by last_accessed_at descending (excluding quarantined files by default).
pub fn list_all_blobs(db: &Database) -> Result<Vec<BlobRecord>> {
    list_all_blobs_filtered(db, false)
}

/// Return blobs in the database, with option to include quarantined items.
pub fn list_all_blobs_filtered(db: &Database, include_quarantined: bool) -> Result<Vec<BlobRecord>> {
    let now = now_unix();
    // Auto-expire any quarantines that have exceeded their duration (7 days)
    {
        let conn = db.lock();
        let _ = conn.execute(
            "UPDATE local_blobs
             SET is_quarantined = 0, quarantine_expires_at = NULL
             WHERE is_quarantined = 1 AND quarantine_expires_at IS NOT NULL AND quarantine_expires_at <= ?1",
            params![now],
        );
    }

    let mut records = {
        let conn = db.lock();
        let sql = if include_quarantined {
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at,
                    written_at, is_quarantined, quarantine_expires_at, catalog_first_seen_at, last_any_seed_seen_at
             FROM local_blobs
             WHERE hash NOT IN (SELECT hash FROM deleted_blobs)
             ORDER BY last_accessed_at DESC"
        } else {
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at,
                    written_at, is_quarantined, quarantine_expires_at, catalog_first_seen_at, last_any_seed_seen_at
             FROM local_blobs
             WHERE (is_quarantined = 0 OR is_quarantined IS NULL)
               AND hash NOT IN (SELECT hash FROM deleted_blobs)
             ORDER BY last_accessed_at DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let raw_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)? as u64,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)? != 0,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<i64>>(8)?.unwrap_or(0) != 0,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, Option<i64>>(11)?,
            ))
        })?;

        let mut list = Vec::new();
        for r in raw_rows {
            let (hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at,
                 written_at, is_quarantined, quarantine_expires_at, catalog_first_seen_at, last_any_seed_seen_at) = r?;
            list.push(BlobRecord {
                hash,
                name,
                size_bytes,
                last_accessed_at,
                is_pinned,
                owner_node_id,
                delete_votes: Vec::new(),
                keep_votes: Vec::new(),
                deletion_scheduled_at,
                seed_count: 1,
                is_cached: false,
                written_at,
                is_quarantined,
                quarantine_expires_at,
                upvotes: 0,
                downvotes: 0,
                net_rating: 0,
                is_starred: false,
                catalog_first_seen_at,
                last_any_seed_seen_at,
            });
        }
        list
    };

    for r in &mut records {
        r.delete_votes = get_delete_votes(db, &r.hash).unwrap_or_default();
        r.keep_votes = get_keep_votes(db, &r.hash).unwrap_or_default();
        r.seed_count = get_seed_count(db, &r.hash, None, false).unwrap_or(1);
        let (up, down, net) = get_file_ratings(db, &r.hash).unwrap_or((0, 0, 0));
        r.upvotes = up;
        r.downvotes = down;
        r.net_rating = net;
        r.is_starred = is_starred(db, &r.hash).unwrap_or(false);
    }

    Ok(records)
}

// ---------------------------------------------------------------------------
// Step 38: Study Ratings, Bookmarks, and Emergency Quarantine
// ---------------------------------------------------------------------------

/// Cast an upvote (+1), downvote (-1), or neutral (0) rating on a file.
pub fn rate_file(db: &Database, hash: &str, voter_node_id: &str, rating: i8) -> Result<()> {
    if ![-1, 0, 1].contains(&rating) {
        anyhow::bail!("Rating must be -1, 0, or 1");
    }
    let now = now_unix();
    let conn = db.lock();
    if rating == 0 {
        conn.execute(
            "DELETE FROM file_ratings WHERE hash = ?1 AND voter_node_id = ?2",
            params![hash, voter_node_id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO file_ratings(hash, voter_node_id, rating, voted_at)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(hash, voter_node_id) DO UPDATE SET rating = excluded.rating, voted_at = excluded.voted_at",
            params![hash, voter_node_id, rating, now],
        )?;
    }

    // Check if quarantined file can be cleared by >= 80% clear votes
    let is_quar: bool = conn
        .query_row(
            "SELECT is_quarantined FROM local_blobs WHERE hash = ?1",
            params![hash],
            |r| r.get::<_, Option<i64>>(0).map(|v| v.unwrap_or(0) != 0),
        )
        .unwrap_or(false);

    if is_quar {
        let clear_votes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM file_ratings WHERE hash = ?1 AND rating = 1",
                params![hash],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let report_votes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM quarantine_reports WHERE hash = ?1",
                params![hash],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let total = clear_votes + report_votes;
        if total > 0 && (clear_votes as f64 / total as f64) >= 0.80 {
            let _ = conn.execute(
                "UPDATE local_blobs SET is_quarantined = 0, quarantine_expires_at = NULL WHERE hash = ?1",
                params![hash],
            );
        }
    }
    Ok(())
}

/// Retrieve ratings count (upvotes, downvotes, net score).
pub fn get_file_ratings(db: &Database, hash: &str) -> Result<(usize, usize, i64)> {
    let conn = db.lock();
    let upvotes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM file_ratings WHERE hash = ?1 AND rating = 1",
            params![hash],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let downvotes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM file_ratings WHERE hash = ?1 AND rating = -1",
            params![hash],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let net = upvotes - downvotes;
    Ok((upvotes as usize, downvotes as usize, net))
}

/// Toggle bookmark / star state for a file (stored locally, not synced to peers).
pub fn toggle_star(db: &Database, hash: &str) -> Result<bool> {
    let conn = db.lock();
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM file_stars WHERE hash = ?1",
            params![hash],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    if exists {
        conn.execute("DELETE FROM file_stars WHERE hash = ?1", params![hash])?;
        Ok(false)
    } else {
        conn.execute(
            "INSERT INTO file_stars(hash, starred_at) VALUES(?1, ?2)",
            params![hash, now_unix()],
        )?;
        Ok(true)
    }
}

/// Check if a file is starred.
pub fn is_starred(db: &Database, hash: &str) -> Result<bool> {
    let conn = db.lock();
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM file_stars WHERE hash = ?1",
            params![hash],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    Ok(exists)
}

/// Report a file as dangerous. Triggers emergency quarantine if quorum of reports arrives in 1 hour.
pub fn report_dangerous(db: &Database, hash: &str, reporter_node_id: &str) -> Result<QuarantineStatus> {
    let now = now_unix();
    let conn = db.lock();
    conn.execute(
        "INSERT OR REPLACE INTO quarantine_reports(hash, reporter_node_id, reported_at) VALUES(?1, ?2, ?3)",
        params![hash, reporter_node_id, now],
    )?;

    // Check distinct reports within the last 1 hour (3600 seconds)
    let count: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT reporter_node_id) FROM quarantine_reports WHERE hash = ?1 AND reported_at > (?2 - 3600)",
        params![hash, now],
        |r| r.get(0),
    )?;

    // Anti-Sybil Defense (VULN-03): Check existing community keep votes or stars
    let keep_votes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM file_keep_votes WHERE hash = ?1",
            params![hash],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // If community nodes actively voted to keep the file, require higher quorum to prevent censorship
    let required_threshold = 5 + keep_votes;

    let is_quarantined = if count >= required_threshold {
        let expires_at = now + 604800; // 7 days
        conn.execute(
            "UPDATE local_blobs SET is_quarantined = 1, quarantine_expires_at = ?1 WHERE hash = ?2",
            params![expires_at, hash],
        )?;
        true
    } else {
        conn.query_row(
            "SELECT is_quarantined FROM local_blobs WHERE hash = ?1",
            params![hash],
            |r| r.get::<_, Option<i64>>(0).map(|v| v.unwrap_or(0) != 0),
        )
        .unwrap_or(false)
    };

    Ok(QuarantineStatus {
        is_quarantined,
        report_count: count as u32,
    })
}

/// Verify folder is empty before deletion.
pub fn check_folder_empty(db: &Database, virtual_path: &str) -> Result<()> {
    let conn = db.lock();
    let clean = virtual_path.trim().trim_matches('/');
    let prefix = format!("{clean}/%");
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM local_blobs WHERE name LIKE ?1",
        params![prefix],
        |r| r.get(0),
    )?;
    if count > 0 {
        anyhow::bail!("Cannot delete a folder that still contains files. Remove all files first.");
    }
    Ok(())
}

/// Create and persist a virtual folder, along with its parent segments.
pub fn create_virtual_folder(db: &Database, virtual_path: &str) -> Result<()> {
    let clean = virtual_path.trim().replace('\\', "/");
    let parts: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(());
    }

    let conn = db.lock();
    let now = now_unix();
    let mut current = String::new();

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            current.push('/');
        }
        current.push_str(part);
        conn.execute(
            "INSERT OR IGNORE INTO virtual_folders (path, created_at) VALUES (?1, ?2)",
            params![current, now],
        )?;
    }
    Ok(())
}

/// List all explicitly persisted virtual folders.
pub fn list_virtual_folders(db: &Database) -> Result<Vec<String>> {
    let conn = db.lock();
    let mut stmt = conn.prepare("SELECT path FROM virtual_folders ORDER BY path ASC")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut folders = Vec::new();
    for f in rows.flatten() {
        folders.push(f);
    }
    Ok(folders)
}

/// Delete an empty virtual folder and any empty subfolders from persistence.
pub fn delete_virtual_folder(db: &Database, virtual_path: &str) -> Result<()> {
    check_folder_empty(db, virtual_path)?;
    delete_virtual_folder_cascade(db, virtual_path)
}

/// Unconditionally remove a virtual folder and any of its child subfolders from the virtual_folders index.
pub fn delete_virtual_folder_cascade(db: &Database, virtual_path: &str) -> Result<()> {
    let clean = virtual_path.trim().trim_matches('/').replace('\\', "/");
    let conn = db.lock();
    let subfolder_prefix = format!("{clean}/%");
    conn.execute(
        "DELETE FROM virtual_folders WHERE path = ?1 OR path LIKE ?2",
        params![clean, subfolder_prefix],
    )?;
    Ok(())
}

/// Delete a virtual folder and all its contents (contained files, subfolders, tombstones for owned files).
/// Returns a tuple of (deleted_hashes, created_tombstones).
pub fn delete_virtual_folder_recursive(
    db: &Database,
    virtual_path: &str,
    our_node_id: Option<&str>,
    secret_key: Option<&iroh::SecretKey>,
) -> Result<(Vec<String>, Vec<DeletionTombstone>)> {
    let clean = virtual_path.trim().trim_matches('/').replace('\\', "/");
    let exact_match = clean.clone();
    let subfolder_prefix = format!("{clean}/%");

    let mut deleted_hashes = Vec::new();
    let mut tombstones = Vec::new();

    {
        let conn = db.lock();
        let mut stmt = conn.prepare(
            "SELECT hash, owner_node_id FROM local_blobs WHERE name = ?1 OR name LIKE ?2",
        )?;
        let rows = stmt.query_map(params![exact_match, subfolder_prefix], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?;

        for row in rows.flatten() {
            let (hash, _owner_id) = row;
            deleted_hashes.push(hash.clone());

            // Generate a signed tombstone for every deleted file so this node never re-accepts it
            if let (Some(our_id), Some(sk)) = (our_node_id, secret_key) {
                let tombstone = DeletionTombstone::create_signed(
                    hash.clone(),
                    our_id.to_string(),
                    now_unix(),
                    sk,
                );
                tombstones.push(tombstone);
            }
        }
    }

    // Persist all generated tombstones locally to prevent peer resurrection
    for t in &tombstones {
        let _ = record_deletion_tombstone_local(db, t);
    }

    // Now delete from local_blobs, file_seeds, and virtual_folders
    let conn = db.lock();
    conn.execute(
        "DELETE FROM local_blobs WHERE name = ?1 OR name LIKE ?2",
        params![exact_match, subfolder_prefix],
    )?;
    for hash in &deleted_hashes {
        let _ = conn.execute("DELETE FROM file_seeds WHERE hash = ?1", params![hash]);
    }
    conn.execute(
        "DELETE FROM virtual_folders WHERE path = ?1 OR path LIKE ?2",
        params![exact_match, subfolder_prefix],
    )?;

    Ok((deleted_hashes, tombstones))
}

// ---------------------------------------------------------------------------
// Step 36: Sole Seed Files Discovery
// ---------------------------------------------------------------------------

/// Return files for which this node is the only known seeder in the swarm.
pub fn get_sole_seed_files(db: &Database, our_node_id: &str) -> Result<Vec<SoleSeedFile>> {
    let now = now_unix();
    let threshold = now - 300; // 5 minutes
    let conn = db.lock();

    let mut stmt = conn.prepare(
        "SELECT b.hash, b.name, b.size_bytes,
                (SELECT GROUP_CONCAT(DISTINCT s.peer_node_id) FROM file_seeds s
                 WHERE s.hash = b.hash AND s.last_seen_at > ?1) AS seeders
         FROM local_blobs b
         WHERE (b.is_quarantined = 0 OR b.is_quarantined IS NULL)",
    )?;

    let rows = stmt.query_map(params![threshold], |r| {
        let hash: String = r.get(0)?;
        let name: Option<String> = r.get(1)?;
        let size_bytes: i64 = r.get(2)?;
        let seeders_str: Option<String> = r.get(3)?;
        Ok((hash, name, size_bytes as u64, seeders_str))
    })?;

    let mut files = Vec::new();
    for row in rows {
        let (hash, name, size_bytes, seeders_str) = row?;
        let mut has_remote_seed = false;
        if let Some(s_str) = seeders_str {
            for seeder in s_str.split(',') {
                let trimmed = seeder.trim();
                if !trimmed.is_empty() && !crate::storage::db::is_matching_node_id(trimmed, our_node_id) {
                    has_remote_seed = true;
                    break;
                }
            }
        }
        if !has_remote_seed {
            files.push(SoleSeedFile {
                hash,
                name,
                size_bytes,
            });
        }
    }
    Ok(files)
}

// ---------------------------------------------------------------------------
// Step 39: Transfer Resume Queue CRUD
// ---------------------------------------------------------------------------

pub fn add_or_update_resume_queue(
    db: &Database,
    hash: &str,
    name: Option<&str>,
    total_bytes: u64,
    received_bytes: u64,
    source_peer: Option<&str>,
) -> Result<()> {
    let now = now_unix();
    let conn = db.lock();
    conn.execute(
        "INSERT INTO transfer_resume_queue(hash, name, total_bytes, received_bytes, source_peer, created_at, updated_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(hash) DO UPDATE SET
             name = COALESCE(excluded.name, transfer_resume_queue.name),
             received_bytes = excluded.received_bytes,
             source_peer = COALESCE(excluded.source_peer, transfer_resume_queue.source_peer),
             updated_at = excluded.updated_at",
        params![hash, name, total_bytes as i64, received_bytes as i64, source_peer, now, now],
    )?;
    Ok(())
}

pub fn update_resume_progress(db: &Database, hash: &str, received_bytes: u64) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "UPDATE transfer_resume_queue SET received_bytes = ?1, updated_at = ?2 WHERE hash = ?3",
        params![received_bytes as i64, now_unix(), hash],
    )?;
    Ok(())
}

pub fn remove_resume_queue(db: &Database, hash: &str) -> Result<()> {
    let conn = db.lock();
    conn.execute("DELETE FROM transfer_resume_queue WHERE hash = ?1", params![hash])?;
    Ok(())
}

pub fn list_resume_queue(db: &Database) -> Result<Vec<TransferQueueItem>> {
    let conn = db.lock();
    let mut stmt = conn.prepare(
        "SELECT hash, name, total_bytes, received_bytes, source_peer, created_at, updated_at
         FROM transfer_resume_queue ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(TransferQueueItem {
            hash: r.get(0)?,
            name: r.get(1)?,
            total_bytes: r.get::<_, i64>(2)? as u64,
            received_bytes: r.get::<_, i64>(3)? as u64,
            source_peer: r.get(4)?,
            created_at: r.get(5)?,
            updated_at: r.get(6)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

// ---------------------------------------------------------------------------
// Step 35: Daily SSD Write Budget Tracker
// ---------------------------------------------------------------------------

pub fn get_today_write_budget(db: &Database, date_key: &str) -> Result<u64> {
    let conn = db.lock();
    let bytes: i64 = conn
        .query_row(
            "SELECT bytes_written FROM write_budget WHERE date_key = ?1",
            params![date_key],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(bytes as u64)
}

pub fn add_to_today_write_budget(db: &Database, date_key: &str, bytes: u64) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO write_budget(date_key, bytes_written) VALUES(?1, ?2)
         ON CONFLICT(date_key) DO UPDATE SET bytes_written = write_budget.bytes_written + excluded.bytes_written",
        params![date_key, bytes as i64],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Step 41: Bilateral Seeding Receipts
// ---------------------------------------------------------------------------

pub fn store_seeding_receipt(
    db: &Database,
    hash: &str,
    seeder_node_id: &str,
    recipient_node_id: &str,
    bytes_transferred: u64,
    issued_at: i64,
    signature: &str,
) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO seeding_receipts(hash, seeder_node_id, recipient_node_id, bytes_transferred, issued_at, signature)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![hash, seeder_node_id, recipient_node_id, bytes_transferred as i64, issued_at, signature],
    )?;
    Ok(())
}

pub fn count_verified_receipts_for_seeder(
    db: &Database,
    seeder_node_id: &str,
    since_timestamp: i64,
) -> Result<usize> {
    let conn = db.lock();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM seeding_receipts
         WHERE seeder_node_id = ?1
           AND issued_at > ?2
           AND recipient_node_id != seeder_node_id",
        params![seeder_node_id, since_timestamp],
        |r| r.get(0),
    )?;
    Ok(count as usize)
}

// ---------------------------------------------------------------------------
// Step 41: Peer Version Tracking & Swarm Median Version
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmVersionInfo {
    pub local_version: String,
    pub median_version: String,
    pub peer_count: usize,
    pub peer_versions: Vec<String>,
}

/// Record or update an announced application version for a swarm peer.
pub fn record_peer_version(db: &Database, node_id: &str, app_version: &str) -> Result<()> {
    let now = now_unix();
    let conn = db.lock();
    let existing: Option<(String, i64)> = conn
        .query_row(
            "SELECT app_version, last_seen_at FROM peer_versions WHERE node_id = ?1",
            params![node_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;

    if let Some((ref ver, last_seen)) = existing {
        if ver == app_version && now - last_seen < 60 {
            return Ok(());
        }
    }

    conn.execute(
        "INSERT INTO peer_versions (node_id, app_version, last_seen_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(node_id) DO UPDATE SET
             app_version  = excluded.app_version,
             last_seen_at = excluded.last_seen_at",
        params![node_id, app_version, now],
    )?;
    Ok(())
}

/// Retrieve all active peer versions (seen in the last 30 minutes).
pub fn get_active_peer_versions(db: &Database) -> Result<Vec<String>> {
    let threshold = now_unix() - 1800; // 30 minutes
    let conn = db.lock();
    let mut stmt = conn.prepare(
        "SELECT app_version FROM peer_versions WHERE last_seen_at >= ?1 ORDER BY app_version ASC"
    )?;
    let rows = stmt.query_map(params![threshold], |r| r.get(0))?;
    let mut versions = Vec::new();
    for v in rows.flatten() {
        versions.push(v);
    }
    Ok(versions)
}

/// Calculate the true median version across the swarm (active peers + local node).
pub fn calculate_swarm_median_version(db: &Database, local_version: &str) -> SwarmVersionInfo {
    let mut all_versions = get_active_peer_versions(db).unwrap_or_default();
    let peer_count = all_versions.len();
    all_versions.push(local_version.to_string());

    // Sort versions using semver if valid, falling back to lexicographical order
    all_versions.sort_by(|a, b| {
        let clean_a = a.trim_start_matches('v');
        let clean_b = b.trim_start_matches('v');
        match (semver::Version::parse(clean_a), semver::Version::parse(clean_b)) {
            (Ok(va), Ok(vb)) => va.cmp(&vb),
            _ => a.cmp(b),
        }
    });

    let median_version = if all_versions.is_empty() {
        local_version.to_string()
    } else {
        let mid = all_versions.len() / 2;
        all_versions[mid].clone()
    };

    SwarmVersionInfo {
        local_version: local_version.to_string(),
        median_version,
        peer_count,
        peer_versions: all_versions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn make_db() -> (TempDir, Arc<Database>) {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(&dir.path().join("test.db")).unwrap());
        (dir, db)
    }

    #[test]
    fn upsert_and_touch() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "hash1", Some("test.pdf"), 1024, false, Some("node1")).unwrap();
        let before = get_blob(&db, "hash1").unwrap().unwrap();
        assert_eq!(before.name.as_deref(), Some("test.pdf"));
        assert_eq!(before.owner_node_id.as_deref(), Some("node1"));
        assert!(before.delete_votes.is_empty());
        std::thread::sleep(std::time::Duration::from_secs(1));
        touch_blob(&db, "hash1").unwrap();
        let after = get_blob(&db, "hash1").unwrap().unwrap();
        assert!(after.last_accessed_at >= before.last_accessed_at);
    }

    #[test]
    fn voting_to_delete_accumulates() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "target_file", Some("file.pdf"), 100, false, Some("creator")).unwrap();
        assert_eq!(cast_delete_vote(&db, "target_file", "peer1").unwrap(), 1);
        assert_eq!(cast_delete_vote(&db, "target_file", "peer2").unwrap(), 2);
        // Duplicate vote from peer2 shouldn't increment
        assert_eq!(cast_delete_vote(&db, "target_file", "peer2").unwrap(), 2);

        let votes = get_delete_votes(&db, "target_file").unwrap();
        assert_eq!(votes.len(), 2);
        assert!(votes.contains(&"peer1".to_string()));
        assert!(votes.contains(&"peer2".to_string()));

        // Remove vote
        assert_eq!(remove_delete_vote(&db, "target_file", "peer1").unwrap(), 1);
        let votes_after = get_delete_votes(&db, "target_file").unwrap();
        assert_eq!(votes_after.len(), 1);
    }

    #[test]
    fn pin_protects_from_listing() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "pinned", None, 100, true, None).unwrap();
        upsert_blob(&db, "unpinned", None, 100, false, None).unwrap();
        let evictable = oldest_unpinned(&db, 100).unwrap();
        assert!(evictable.iter().all(|b| b.hash != "pinned"));
    }

    #[test]
    fn totals_are_correct() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "a", None, 500, false, None).unwrap();
        upsert_blob(&db, "b", None, 300, true, None).unwrap();
        assert_eq!(total_unpinned_bytes(&db).unwrap(), 500);
        assert_eq!(total_pinned_bytes(&db).unwrap(), 300);
        assert_eq!(total_bytes(&db).unwrap(), 800);
    }

    #[test]
    fn rename_blob_moves_path() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "hash_move", Some("OldFolder/notes.txt"), 100, false, None).unwrap();
        let before = get_blob(&db, "hash_move").unwrap().unwrap();
        assert_eq!(before.name.as_deref(), Some("OldFolder/notes.txt"));

        rename_blob(&db, "hash_move", "NewFolder/notes.txt").unwrap();
        let after = get_blob(&db, "hash_move").unwrap().unwrap();
        assert_eq!(after.name.as_deref(), Some("NewFolder/notes.txt"));
    }

    #[test]
    fn seed_count_tracks_peer_and_sole_seeds() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "hash_seeds", Some("file.pdf"), 200, false, None).unwrap();

        // No peers yet: sole seed = 0 remote seeds, but 1 total if local
        assert_eq!(get_seed_count(&db, "hash_seeds", Some("my_node_id"), false).unwrap(), 0);
        assert_eq!(get_seed_count(&db, "hash_seeds", Some("my_node_id"), true).unwrap(), 1);

        // Record peer seed
        record_peer_seed(&db, "hash_seeds", "peer_node_1").unwrap();
        assert_eq!(get_seed_count(&db, "hash_seeds", Some("my_node_id"), false).unwrap(), 1);
        assert_eq!(get_seed_count(&db, "hash_seeds", Some("my_node_id"), true).unwrap(), 2);

        // Record local node seed - should be excluded from peer count to prevent double counting
        record_peer_seed(&db, "hash_seeds", "my_node_id").unwrap();
        assert_eq!(get_seed_count(&db, "hash_seeds", Some("my_node_id"), false).unwrap(), 1);
        assert_eq!(get_seed_count(&db, "hash_seeds", Some("my_node_id"), true).unwrap(), 2);
    }

    #[test]
    fn keep_votes_and_grace_period_evaluation() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "target_vote", Some("document.pdf"), 500, false, Some("creator")).unwrap();

        // 1 delete vote: under quorum of 3 -> Safe
        cast_delete_vote(&db, "target_vote", "p1").unwrap();
        let b1 = get_blob(&db, "target_vote").unwrap().unwrap();
        assert_eq!(b1.delete_votes.len(), 1);
        assert_eq!(b1.deletion_scheduled_at, None);

        // 2 delete votes: still under quorum -> Safe
        cast_delete_vote(&db, "target_vote", "p2").unwrap();
        let b2 = get_blob(&db, "target_vote").unwrap().unwrap();
        assert_eq!(b2.deletion_scheduled_at, None);

        // 3 delete votes, 0 keep votes -> 100% delete ratio (>= 86%), triggers grace period!
        cast_delete_vote(&db, "target_vote", "p3").unwrap();
        let b3 = get_blob(&db, "target_vote").unwrap().unwrap();
        assert!(b3.deletion_scheduled_at.is_some());

        // Cast 1 keep vote: now 3 delete, 1 keep -> 3/4 = 75% delete ratio (< 86%) -> Grace period cancelled!
        cast_keep_vote(&db, "target_vote", "defender1").unwrap();
        let b4 = get_blob(&db, "target_vote").unwrap().unwrap();
        assert_eq!(b4.keep_votes.len(), 1);
        assert_eq!(b4.deletion_scheduled_at, None);

        // Defender retracts keep vote -> 3 delete, 0 keep -> 100% -> Grace period re-triggers!
        remove_keep_vote(&db, "target_vote", "defender1").unwrap();
        let b5 = get_blob(&db, "target_vote").unwrap().unwrap();
        assert!(b5.deletion_scheduled_at.is_some());
    }

    #[test]
    fn path_conflict_enforcement() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "hash_a", Some("docs/lecture1.pdf"), 100, false, None).unwrap();

        // Same path, different hash -> Err
        let err = upsert_blob(&db, "hash_b", Some("docs/lecture1.pdf"), 200, false, None);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("already occupied"));

        // Same path, identical hash -> Ok(())
        let ok = upsert_blob(&db, "hash_a", Some("docs/lecture1.pdf"), 100, false, None);
        assert!(ok.is_ok());
    }

    #[test]
    fn ratings_and_stars_and_quarantine() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "hash_q", Some("suspicious.pdf"), 500, false, None).unwrap();

        // Stars
        assert!(!is_starred(&db, "hash_q").unwrap());
        assert!(toggle_star(&db, "hash_q").unwrap());
        assert!(is_starred(&db, "hash_q").unwrap());
        assert!(!toggle_star(&db, "hash_q").unwrap());
        assert!(!is_starred(&db, "hash_q").unwrap());

        // Ratings
        rate_file(&db, "hash_q", "student1", 1).unwrap();
        rate_file(&db, "hash_q", "student2", 1).unwrap();
        rate_file(&db, "hash_q", "student3", -1).unwrap();
        let (up, down, net) = get_file_ratings(&db, "hash_q").unwrap();
        assert_eq!(up, 2);
        assert_eq!(down, 1);
        assert_eq!(net, 1);

        // Quarantine reports
        // 4 reports in 1 hour -> not quarantined
        for i in 1..=4 {
            let res = report_dangerous(&db, "hash_q", &format!("reporter_{i}")).unwrap();
            assert_eq!(res.report_count, i);
            assert!(!res.is_quarantined);
        }
        let unquar_blobs = list_all_blobs(&db).unwrap();
        assert_eq!(unquar_blobs.len(), 1);

        // 5th report -> triggers quarantine!
        let res5 = report_dangerous(&db, "hash_q", "reporter_5").unwrap();
        assert_eq!(res5.report_count, 5);
        assert!(res5.is_quarantined);

        // Filtered list_all_blobs hides quarantined file
        let regular_blobs = list_all_blobs(&db).unwrap();
        assert!(regular_blobs.is_empty());

        // But list_all_blobs_filtered(include_quarantined = true) shows it
        let all_blobs = list_all_blobs_filtered(&db, true).unwrap();
        assert_eq!(all_blobs.len(), 1);
        assert!(all_blobs[0].is_quarantined);
    }

    #[test]
    fn folder_empty_check() {
        let (_dir, db) = make_db();
        upsert_blob(&db, "file_inside", Some("Math101/Calculus/lecture1.pdf"), 100, false, None).unwrap();

        // Math101 is non-empty -> Err
        assert!(check_folder_empty(&db, "Math101").is_err());
        assert!(check_folder_empty(&db, "Math101/Calculus").is_err());

        // Physics is empty -> Ok(())
        assert!(check_folder_empty(&db, "Physics").is_ok());
    }

    #[test]
    fn virtual_folders_crud_and_persistence() {
        let (_dir, db) = make_db();

        // Create nested empty folder: "CS50/Lectures/Week1"
        create_virtual_folder(&db, "CS50/Lectures/Week1").unwrap();
        let folders = list_virtual_folders(&db).unwrap();
        // Should create CS50, CS50/Lectures, and CS50/Lectures/Week1
        assert!(folders.contains(&"CS50".to_string()));
        assert!(folders.contains(&"CS50/Lectures".to_string()));
        assert!(folders.contains(&"CS50/Lectures/Week1".to_string()));

        // Delete empty folder
        delete_virtual_folder(&db, "CS50/Lectures/Week1").unwrap();
        let after_delete = list_virtual_folders(&db).unwrap();
        assert!(!after_delete.contains(&"CS50/Lectures/Week1".to_string()));
        assert!(after_delete.contains(&"CS50/Lectures".to_string()));

        // Place a file inside CS50/Lectures
        upsert_blob(&db, "f_hash", Some("CS50/Lectures/slide.pdf"), 200, false, None).unwrap();

        // Attempting to delete CS50/Lectures should now fail because it's non-empty
        assert!(delete_virtual_folder(&db, "CS50/Lectures").is_err());

        // But recursive deletion succeeds, removes files and folder, and issues tombstones for owned files
        let (deleted, tombstones) = delete_virtual_folder_recursive(&db, "CS50", None, None).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0], "f_hash");
        assert!(tombstones.is_empty()); // No key passed
        assert!(get_blob(&db, "f_hash").unwrap().is_none());
        assert!(!list_virtual_folders(&db).unwrap().contains(&"CS50".to_string()));
    }

    #[test]
    fn transfer_queue_and_receipts_and_budget() {
        let (_dir, db) = make_db();

        // Write budget
        assert_eq!(get_today_write_budget(&db, "2026-11-04").unwrap(), 0);
        add_to_today_write_budget(&db, "2026-11-04", 1024).unwrap();
        assert_eq!(get_today_write_budget(&db, "2026-11-04").unwrap(), 1024);

        // Transfer queue
        add_or_update_resume_queue(&db, "resume_hash", Some("large.iso"), 1000, 200, Some("peer_a")).unwrap();
        let items = list_resume_queue(&db).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].received_bytes, 200);

        update_resume_progress(&db, "resume_hash", 600).unwrap();
        let items_updated = list_resume_queue(&db).unwrap();
        assert_eq!(items_updated[0].received_bytes, 600);

        remove_resume_queue(&db, "resume_hash").unwrap();
        assert!(list_resume_queue(&db).unwrap().is_empty());

        // Seeding receipts
        store_seeding_receipt(&db, "receipt_blob", "seeder_1", "recipient_2", 5000, 1000, "sig123").unwrap();
        let count = count_verified_receipts_for_seeder(&db, "seeder_1", 500).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_swarm_median_version() {
        let (_dir, db) = make_db();
        // Record multiple peer versions
        record_peer_version(&db, "peer_old", "0.1.9").unwrap();
        record_peer_version(&db, "peer_new1", "0.1.12").unwrap();
        record_peer_version(&db, "peer_new2", "0.2.0").unwrap();

        // Local version is 0.1.11. Sorted list: [0.1.9, 0.1.11, 0.1.12, 0.2.0]
        // Mid index is 4 / 2 = 2 -> 0.1.12
        let info = calculate_swarm_median_version(&db, "0.1.11");
        assert_eq!(info.median_version, "0.1.12");
        assert_eq!(info.peer_count, 3);
    }

    #[test]
    fn test_deletion_tombstone_prevents_resurrection_and_rejects_spoofed_owner() {
        let (_dir, db) = make_db();

        let owner_sk = iroh::SecretKey::generate();
        let owner_id = owner_sk.public().to_string();

        let attacker_sk = iroh::SecretKey::generate();
        let attacker_id = attacker_sk.public().to_string();

        let hash = "deadbeef000102030405060708090a0b0c0d0e0f101112131415161718191a1b";

        // 1. Owner adds file
        upsert_blob(&db, hash, Some("important_lecture.pdf"), 4096, false, Some(&owner_id)).unwrap();
        assert!(get_blob(&db, hash).unwrap().is_some());

        // 2. Attacker Alice attempts to spoof deletion with attacker's key under owner's ID
        let mut forged_tombstone = DeletionTombstone::create_signed(
            hash.to_string(),
            attacker_id.clone(),
            now_unix(),
            &attacker_sk,
        );
        // Attacker claims the tombstone belongs to owner
        forged_tombstone.owner_node_id = owner_id.clone();
        assert!(!forged_tombstone.verify(), "Forged signature under another node_id must fail verification");
        assert!(!record_deletion_tombstone(&db, &forged_tombstone).unwrap(), "Database must reject forged tombstone");
        assert!(get_blob(&db, hash).unwrap().is_some(), "File must still be present");

        // 3. Attacker signs with attacker's key under attacker's own ID and tries to delete Bob's file
        let unauthorized_tombstone = DeletionTombstone::create_signed(
            hash.to_string(),
            attacker_id.clone(),
            now_unix(),
            &attacker_sk,
        );
        assert!(unauthorized_tombstone.verify(), "Valid signature for attacker");
        assert!(!record_deletion_tombstone(&db, &unauthorized_tombstone).unwrap(), "Database must reject non-owner deletion");
        assert!(get_blob(&db, hash).unwrap().is_some(), "File must still be present");

        // 4. Genuine owner signs deletion tombstone
        let genuine_tombstone = DeletionTombstone::create_signed(
            hash.to_string(),
            owner_id.clone(),
            now_unix(),
            &owner_sk,
        );
        assert!(genuine_tombstone.verify());
        assert!(record_deletion_tombstone(&db, &genuine_tombstone).unwrap(), "Owner tombstone must be accepted");
        assert!(get_blob(&db, hash).unwrap().is_none(), "File must be deleted from tracker");
        assert!(is_blob_tombstoned(&db, hash).unwrap(), "Hash must be marked tombstoned");

        // 5. Another peer sends a CatalogDump or FileAdded containing the deleted file
        upsert_blob(&db, hash, Some("important_lecture.pdf"), 4096, false, Some(&owner_id)).unwrap();
        assert!(get_blob(&db, hash).unwrap().is_none(), "Tombstone must strictly prevent resurrection");
    }
}
