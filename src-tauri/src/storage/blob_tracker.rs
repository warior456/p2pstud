// src-tauri/src/storage/blob_tracker.rs
// CRUD operations on the local_blobs SQLite table and file_delete_votes table.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::params;
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

/// A row from the local_blobs table with associated community delete votes, keep votes, and seed count.
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
}

fn default_seed_count() -> usize {
    1
}

fn now_unix() -> i64 {
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
    conn.execute(
        "INSERT OR REPLACE INTO file_seeds(hash, peer_node_id, last_seen_at) VALUES(?1, ?2, ?3)",
        params![hash, peer_node_id, now_unix()],
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
    let peer_seeds: i64 = if let Some(my_id) = local_node_id {
        conn.query_row(
            "SELECT COUNT(DISTINCT peer_node_id) FROM file_seeds WHERE hash = ?1 AND peer_node_id != ?2 AND last_seen_at >= ?3",
            params![hash, my_id, threshold],
            |r| r.get(0),
        )
        .unwrap_or(0)
    } else {
        conn.query_row(
            "SELECT COUNT(DISTINCT peer_node_id) FROM file_seeds WHERE hash = ?1 AND last_seen_at >= ?2",
            params![hash, threshold],
            |r| r.get(0),
        )
        .unwrap_or(0)
    };

    let mut total = peer_seeds as usize;
    if is_locally_cached {
        total += 1;
    }
    Ok(total)
}

/// Insert or replace a blob record (upsert).
pub fn upsert_blob(
    db: &Database,
    hash: &str,
    name: Option<&str>,
    size_bytes: u64,
    is_pinned: bool,
    owner_node_id: Option<&str>,
) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO local_blobs(hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(hash) DO UPDATE SET
             name             = COALESCE(excluded.name, local_blobs.name),
             size_bytes       = excluded.size_bytes,
             last_accessed_at = excluded.last_accessed_at,
             owner_node_id    = COALESCE(excluded.owner_node_id, local_blobs.owner_node_id)",
        params![
            hash,
            name,
            size_bytes as i64,
            now_unix(),
            is_pinned as i64,
            owner_node_id
        ],
    )?;
    Ok(())
}

/// Update last_accessed_at to now (called when a blob is read/opened).
pub fn touch_blob(db: &Database, hash: &str) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "UPDATE local_blobs SET last_accessed_at = ?1 WHERE hash = ?2",
        params![now_unix(), hash],
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
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at
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
            ))
        })?;

        let mut list = Vec::new();
        for r in raw_rows {
            let (hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at) = r?;
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
            });
        }
        list
    };

    for r in &mut records {
        r.delete_votes = get_delete_votes(db, &r.hash).unwrap_or_default();
        r.keep_votes = get_keep_votes(db, &r.hash).unwrap_or_default();
        r.seed_count = get_seed_count(db, &r.hash, None, false).unwrap_or(1);
    }

    Ok(records)
}

/// Look up a single blob by hash.
pub fn get_blob(db: &Database, hash: &str) -> Result<Option<BlobRecord>> {
    let result = {
        let conn = db.lock();
        conn.query_row(
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at
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
                ))
            },
        )
    };

    let (hash_val, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at) = match result {
        Ok(vals) => vals,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let delete_votes = get_delete_votes(db, &hash_val).unwrap_or_default();
    let keep_votes = get_keep_votes(db, &hash_val).unwrap_or_default();
    let seed_count = get_seed_count(db, &hash_val, None, false).unwrap_or(1);

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
    }))
}

/// Return all blobs in the database ordered by last_accessed_at descending.
pub fn list_all_blobs(db: &Database) -> Result<Vec<BlobRecord>> {
    let mut records = {
        let conn = db.lock();
        let mut stmt = conn.prepare(
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at
             FROM local_blobs
             ORDER BY last_accessed_at DESC",
        )?;
        let raw_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)? as u64,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)? != 0,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<i64>>(6)?,
            ))
        })?;

        let mut list = Vec::new();
        for r in raw_rows {
            let (hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id, deletion_scheduled_at) = r?;
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
            });
        }
        list
    };

    for r in &mut records {
        r.delete_votes = get_delete_votes(db, &r.hash).unwrap_or_default();
        r.keep_votes = get_keep_votes(db, &r.hash).unwrap_or_default();
        r.seed_count = get_seed_count(db, &r.hash, None, false).unwrap_or(1);
    }

    Ok(records)
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
}
