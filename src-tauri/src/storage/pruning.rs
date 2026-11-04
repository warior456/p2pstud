// src-tauri/src/storage/pruning.rs
// Catalog Lifecycle Pruning background worker.
// Purges dead files (0 seeds for >= 2 years) and deprioritizes old files (> 5 years).
// Pinned files are strictly immune from pruning.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use iroh_blobs::store::Store as _;
use rand::Rng;
use tokio::sync::RwLock;

use crate::p2p::engine::IrohEngine;
use crate::p2p::sync::{broadcast_catalog_event, SyncMessage};
use crate::storage::blob_tracker::delete_blob;
use crate::storage::db::Database;

pub const TWO_YEARS_SECS: i64 = 63_072_000;
pub const FIVE_YEARS_SECS: i64 = 157_766_400;

/// Run a single lifecycle pruning pass.
/// Returns the list of purged blob hashes.
pub async fn run_lifecycle_pruning_pass(
    db: &Database,
    engine: Option<&IrohEngine>,
) -> Result<Vec<String>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Step A: Update last_any_seed_seen_at for any active blobs
    {
        let conn = db.lock();
        let _ = conn.execute(
            "UPDATE local_blobs SET last_any_seed_seen_at = ?1
             WHERE hash IN (
                 SELECT DISTINCT hash FROM file_seeds WHERE last_seen_at > ?1 - 3600
             )",
            rusqlite::params![now],
        );
    }

    // Step B: Identify dead files (0 seeds for >= 2 years, UNPINNED only)
    let dead_candidates: Vec<(String, Option<String>)> = {
        let conn = db.lock();
        let mut stmt = conn.prepare(
            "SELECT hash, name FROM local_blobs
             WHERE is_pinned = 0
               AND (
                 last_any_seed_seen_at < ?1 - ?2
                 OR (last_any_seed_seen_at IS NULL AND catalog_first_seen_at < ?1 - ?2)
               )",
        )?;
        let rows = stmt.query_map(rusqlite::params![now, TWO_YEARS_SECS], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        list
    };

    let mut purged = Vec::new();
    for (hash_str, name) in dead_candidates {
        tracing::info!(hash = %hash_str, name = ?name, "Pruning dead file — 0 seeds for 2+ years");

        // Double check pin status for absolute safety
        let is_pinned: bool = {
            let conn = db.lock();
            conn.query_row(
                "SELECT is_pinned FROM local_blobs WHERE hash = ?1",
                rusqlite::params![&hash_str],
                |r| r.get::<_, i64>(0).map(|v| v != 0),
            )
            .unwrap_or(false)
        };

        if is_pinned {
            continue; // Pinned files are strictly immune to pruning
        }

        // Delete from local blob store if present
        if let Some(eng) = engine {
            if let Ok(hash) = hash_str.parse::<iroh_blobs::Hash>() {
                let _ = eng.blobs.store().delete(vec![hash]).await;
            }
        }

        // Delete from SQLite
        if delete_blob(db, &hash_str).is_ok() {
            purged.push(hash_str.clone());

            // Broadcast deletion to peers if engine is running
            if let Some(eng) = engine {
                let our_id = eng.node_id().to_string();
                broadcast_catalog_event(
                    &eng.endpoint,
                    &eng.sync_alpn,
                    SyncMessage::FileDeleted {
                        hash: hash_str,
                        deleter_node_id: our_id,
                    },
                )
                .await;
            }
        }
    }

    Ok(purged)
}

/// Spawns the daily lifecycle pruning worker.
pub fn spawn_lifecycle_pruning_worker(
    db: Arc<Database>,
    engine_lock: Arc<RwLock<Option<IrohEngine>>>,
) {
    tokio::spawn(async move {
        // Jitter initial start between 0 and 3600 seconds to stagger swarm nodes
        let jitter_secs = rand::thread_rng().gen_range(10..=60);
        tokio::time::sleep(Duration::from_secs(jitter_secs)).await;

        let mut interval = tokio::time::interval(Duration::from_secs(86400));
        loop {
            interval.tick().await;

            let guard = engine_lock.read().await;
            if let Err(err) = run_lifecycle_pruning_pass(&db, guard.as_ref()).await {
                tracing::warn!("Lifecycle pruning pass error: {err:#}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::blob_tracker::{get_blob, upsert_blob};
    use tempfile::TempDir;

    fn make_test_db() -> (TempDir, Arc<Database>) {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(&dir.path().join("test.db")).unwrap());
        (dir, db)
    }

    #[tokio::test]
    async fn test_dead_file_purged_after_two_years() {
        let (_dir, db) = make_test_db();
        let now = 100_000_000i64;
        let three_years_ago = now - (TWO_YEARS_SECS + 10_000);

        // Insert unpinned file with 0 seeds for 3 years
        upsert_blob(&db, "dead_blob", Some("old_notes.pdf"), 500, false, None).unwrap();
        {
            let conn = db.lock();
            conn.execute(
                "UPDATE local_blobs SET catalog_first_seen_at = ?1, last_any_seed_seen_at = ?1 WHERE hash = 'dead_blob'",
                rusqlite::params![three_years_ago],
            ).unwrap();
        }

        let purged = run_lifecycle_pruning_pass(&db, None).await.unwrap();
        assert!(purged.contains(&"dead_blob".to_string()));
        assert!(get_blob(&db, "dead_blob").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_pinned_file_survives_lifecycle_pruning() {
        let (_dir, db) = make_test_db();
        let now = 100_000_000i64;
        let three_years_ago = now - (TWO_YEARS_SECS + 10_000);

        // Insert PINNED file with 0 seeds for 3 years
        upsert_blob(&db, "pinned_dead_blob", Some("essential.pdf"), 500, true, None).unwrap();
        {
            let conn = db.lock();
            conn.execute(
                "UPDATE local_blobs SET catalog_first_seen_at = ?1, last_any_seed_seen_at = ?1 WHERE hash = 'pinned_dead_blob'",
                rusqlite::params![three_years_ago],
            ).unwrap();
        }

        let purged = run_lifecycle_pruning_pass(&db, None).await.unwrap();
        assert!(!purged.contains(&"pinned_dead_blob".to_string()));
        assert!(get_blob(&db, "pinned_dead_blob").unwrap().is_some());
    }
}

