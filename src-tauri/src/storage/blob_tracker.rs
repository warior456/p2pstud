// src-tauri/src/storage/blob_tracker.rs
// CRUD operations on the local_blobs SQLite table.

// tracking local blobs
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::db::Database;

/// A row from the local_blobs table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobRecord {
    pub hash: String,
    pub name: Option<String>,
    pub size_bytes: u64,
    pub last_accessed_at: i64,
    pub is_pinned: bool,
    pub owner_node_id: Option<String>,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX_EPOCH")
        .as_secs() as i64
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

/// Delete a blob record (called by the eviction worker after removing from store).
pub fn delete_blob(db: &Database, hash: &str) -> Result<()> {
    let conn = db.lock();
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
    let conn = db.lock();
    let mut stmt = conn.prepare(
        "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id
         FROM local_blobs
         WHERE is_pinned = 0
         ORDER BY last_accessed_at ASC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(BlobRecord {
            hash: row.get(0)?,
            name: row.get(1)?,
            size_bytes: row.get::<_, i64>(2)? as u64,
            last_accessed_at: row.get(3)?,
            is_pinned: row.get::<_, i64>(4)? != 0,
            owner_node_id: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Look up a single blob by hash.
pub fn get_blob(db: &Database, hash: &str) -> Result<Option<BlobRecord>> {
    let conn = db.lock();
    let result = conn.query_row(
        "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id
         FROM local_blobs WHERE hash = ?1",
        params![hash],
        |row| {
            Ok(BlobRecord {
                hash: row.get(0)?,
                name: row.get(1)?,
                size_bytes: row.get::<_, i64>(2)? as u64,
                last_accessed_at: row.get(3)?,
                is_pinned: row.get::<_, i64>(4)? != 0,
                owner_node_id: row.get(5)?,
            })
        },
    );
    match result {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Return all blobs in the database ordered by last_accessed_at descending.
pub fn list_all_blobs(db: &Database) -> Result<Vec<BlobRecord>> {
    let conn = db.lock();
    let mut stmt = conn.prepare(
        "SELECT hash, name, size_bytes, last_accessed_at, is_pinned, owner_node_id
         FROM local_blobs
         ORDER BY last_accessed_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(BlobRecord {
            hash: row.get(0)?,
            name: row.get(1)?,
            size_bytes: row.get::<_, i64>(2)? as u64,
            last_accessed_at: row.get(3)?,
            is_pinned: row.get::<_, i64>(4)? != 0,
            owner_node_id: row.get(5)?,
        })
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
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
        std::thread::sleep(std::time::Duration::from_secs(1));
        touch_blob(&db, "hash1").unwrap();
        let after = get_blob(&db, "hash1").unwrap().unwrap();
        assert!(after.last_accessed_at >= before.last_accessed_at);
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
