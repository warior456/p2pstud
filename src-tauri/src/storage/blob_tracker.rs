// src-tauri/src/storage/blob_tracker.rs
// CRUD operations on the local_blobs SQLite table and file_delete_votes table.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::db::Database;

/// Number of community delete votes required to purge an unwanted/orphaned file.
pub const DELETE_VOTE_QUORUM: usize = 3;

/// A row from the local_blobs table with associated community delete votes and seed count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobRecord {
    pub hash: String,
    pub name: Option<String>,
    pub size_bytes: u64,
    pub last_accessed_at: i64,
    pub is_pinned: bool,
    pub owner_node_id: Option<String>,
    pub delete_votes: Vec<String>,
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

/// Cast a vote to delete a file. Returns the new total vote count for this file.
pub fn cast_delete_vote(db: &Database, hash: &str, voter_node_id: &str) -> Result<usize> {
    let conn = db.lock();
    conn.execute(
        "INSERT OR IGNORE INTO file_delete_votes(hash, voter_node_id, voted_at) VALUES(?1, ?2, ?3)",
        params![hash, voter_node_id, now_unix()],
    )?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_delete_votes WHERE hash = ?1",
        params![hash],
        |r| r.get(0),
    )?;
    Ok(count as usize)
}

/// Retract a delete vote. Returns the new total vote count for this file.
pub fn remove_delete_vote(db: &Database, hash: &str, voter_node_id: &str) -> Result<usize> {
    let conn = db.lock();
    conn.execute(
        "DELETE FROM file_delete_votes WHERE hash = ?1 AND voter_node_id = ?2",
        params![hash, voter_node_id],
    )?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_delete_votes WHERE hash = ?1",
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
pub fn get_seed_count(db: &Database, hash: &str, is_locally_cached: bool) -> Result<usize> {
    let conn = db.lock();
    let threshold = now_unix() - 1800; // 30 minutes
    let peer_seeds: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT peer_node_id) FROM file_seeds WHERE hash = ?1 AND last_seen_at >= ?2",
            params![hash, threshold],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut total = peer_seeds as usize;
    if is_locally_cached {
        total += 1;
    }
    // Every existing file has at least 1 seed (the uploader/origin node)
    if total == 0 {
        total = 1;
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

/// Set or clear the pinned flag on a blob.
pub fn set_pinned(db: &Database, hash: &str, pinned: bool) -> Result<()> {
    let conn = db.lock();
    conn.execute(
        "UPDATE local_blobs SET is_pinned = ?1 WHERE hash = ?2",
        params![pinned as i64, hash],
    )?;
    Ok(())
}

/// Delete a blob record and all its associated delete votes and seed records.
pub fn delete_blob(db: &Database, hash: &str) -> Result<()> {
    let conn = db.lock();
    let _ = conn.execute("DELETE FROM file_delete_votes WHERE hash = ?1", params![hash]);
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
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id
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
            ))
        })?;

        let mut list = Vec::new();
        for r in raw_rows {
            let (hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id) = r?;
            list.push(BlobRecord {
                hash,
                name,
                size_bytes,
                last_accessed_at,
                is_pinned,
                owner_node_id,
                delete_votes: Vec::new(),
                seed_count: 1,
                is_cached: false,
            });
        }
        list
    };

    for r in &mut records {
        r.delete_votes = get_delete_votes(db, &r.hash).unwrap_or_default();
        r.seed_count = get_seed_count(db, &r.hash, false).unwrap_or(1);
    }

    Ok(records)
}

/// Look up a single blob by hash.
pub fn get_blob(db: &Database, hash: &str) -> Result<Option<BlobRecord>> {
    let result = {
        let conn = db.lock();
        conn.query_row(
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id
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
                ))
            },
        )
    };

    let (hash_val, name, size_bytes, last_accessed_at, is_pinned, owner_node_id) = match result {
        Ok(vals) => vals,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let delete_votes = get_delete_votes(db, &hash_val).unwrap_or_default();
    let seed_count = get_seed_count(db, &hash_val, false).unwrap_or(1);

    Ok(Some(BlobRecord {
        hash: hash_val,
        name,
        size_bytes,
        last_accessed_at,
        is_pinned,
        owner_node_id,
        delete_votes,
        seed_count,
        is_cached: false,
    }))
}

/// Return all blobs in the database ordered by last_accessed_at descending.
pub fn list_all_blobs(db: &Database) -> Result<Vec<BlobRecord>> {
    let mut records = {
        let conn = db.lock();
        let mut stmt = conn.prepare(
            "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id
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
            ))
        })?;

        let mut list = Vec::new();
        for r in raw_rows {
            let (hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id) = r?;
            list.push(BlobRecord {
                hash,
                name,
                size_bytes,
                last_accessed_at,
                is_pinned,
                owner_node_id,
                delete_votes: Vec::new(),
                seed_count: 1,
                is_cached: false,
            });
        }
        list
    };

    for r in &mut records {
        r.delete_votes = get_delete_votes(db, &r.hash).unwrap_or_default();
        r.seed_count = get_seed_count(db, &r.hash, false).unwrap_or(1);
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
}
