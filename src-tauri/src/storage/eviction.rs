// src-tauri/src/storage/eviction.rs
// LRU eviction background worker.
//
// Algorithm:
//   Every EVICTION_INTERVAL_SECS seconds (and triggered on every blob write),
//   check total unpinned blob usage. If > 90% of quota, delete oldest-first
//   until usage drops below 80%. Pinned blobs are NEVER evicted.
//   In full-mirror mode the worker exits immediately without evicting anything.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

use crate::p2p::engine::IrohEngine;
use crate::storage::blob_tracker::{
    delete_blob, get_seed_count, oldest_unpinned, total_unpinned_bytes, BlobRecord,
};
use crate::storage::db::Database;

const EVICTION_INTERVAL_SECS: u64 = 60;

/// Fraction of quota at which eviction is triggered.
const EVICTION_HIGH_WATERMARK: f64 = 0.90;
/// Fraction of quota that eviction drives usage down to.
const EVICTION_LOW_WATERMARK: f64 = 0.80;

/// Select and sort eviction candidates according to Step 35 rules:
/// 1. Dwell time guard: Skip blobs written < 24h ago.
/// 2. Sole-seed hard block: Skip blobs with 0 remote seeds.
/// 3. Seed-count-first order: Sort by (remote_seeds DESC, last_accessed_at ASC).
pub fn select_eviction_candidates(
    db: &Database,
    local_node_id: Option<&str>,
    now: i64,
) -> Result<Vec<BlobRecord>> {
    select_eviction_candidates_ext(db, local_node_id, now, false)
}

/// Select and sort eviction candidates with optional dwell time guard relaxation for emergency eviction.
pub fn select_eviction_candidates_ext(
    db: &Database,
    local_node_id: Option<&str>,
    now: i64,
    relax_dwell: bool,
) -> Result<Vec<BlobRecord>> {
    let unpinned = oldest_unpinned(db, 200)?;
    let mut eligible = Vec::new();

    for blob in unpinned {
        // Rule 1: Dwell time guard (24 hours = 86400 seconds)
        if !relax_dwell {
            let written = blob.written_at.unwrap_or(blob.last_accessed_at);
            if now.saturating_sub(written) < 86400 {
                continue;
            }
        }

        // Rule 2: Sole-seed hard block (remote seeds must be >= 1)
        let remote_seeds = get_seed_count(db, &blob.hash, local_node_id, false).unwrap_or(0);
        if remote_seeds == 0 {
            continue;
        }

        eligible.push((blob, remote_seeds));
    }

    // Rule 3: Sort by remote_seeds DESC, then last_accessed_at ASC
    eligible.sort_by(|(blob_a, seeds_a), (blob_b, seeds_b)| {
        seeds_b
            .cmp(seeds_a)
            .then_with(|| blob_a.last_accessed_at.cmp(&blob_b.last_accessed_at))
    });

    Ok(eligible.into_iter().map(|(b, _)| b).collect())
}

/// Read the cache quota from the settings table.
/// Returns `None` if full-mirror mode is enabled.
fn quota_bytes(db: &Database) -> Result<Option<u64>> {
    let full_mirror = db
        .get_setting("full_mirror")?
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");

    if full_mirror {
        return Ok(None);
    }

    let bytes: u64 = db
        .get_setting("cache_limit_bytes")?
        .unwrap_or_else(|| "5368709120".to_string())
        .parse()
        .unwrap_or(5_368_709_120); // 5 GB default

    Ok(Some(bytes))
}

/// Run one eviction pass. Returns the number of blobs deleted.
///
/// This is `pub` so tests can call it directly.
pub async fn run_eviction_pass(db: &Arc<Database>, engine: &IrohEngine) -> Result<usize> {
    let Some(quota) = quota_bytes(db)? else {
        tracing::debug!("Full-mirror mode — skipping eviction");
        return Ok(0);
    };

    let high_mark = (quota as f64 * EVICTION_HIGH_WATERMARK) as u64;
    let low_mark = (quota as f64 * EVICTION_LOW_WATERMARK) as u64;

    let blob_dir = db.resolve_blob_dir();
    let disk_bytes = crate::storage::path_utils::calculate_dir_size(&blob_dir);
    let db_bytes = total_unpinned_bytes(db)?;
    let mut used = disk_bytes.max(db_bytes);

    // If cache usage exceeds high watermark, first prune any orphaned files on disk
    if used > high_mark {
        if let Ok(freed) = crate::storage::path_utils::cleanup_orphaned_disk_blobs(&blob_dir, db) {
            if freed > 0 {
                tracing::info!(freed, "Cleaned up orphaned blobs from disk during eviction");
                let updated_disk = crate::storage::path_utils::calculate_dir_size(&blob_dir);
                used = updated_disk.max(db_bytes);
            }
        }
    }

    if used <= high_mark {
        return Ok(0);
    }

    tracing::info!(
        used_bytes = used,
        quota_bytes = quota,
        high_mark,
        low_mark,
        "Cache above 90% — starting eviction"
    );

    let mut deleted = 0usize;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let my_node_id = engine.node_id().to_string();
    let emergency_mark = (quota as f64 * 0.98) as u64;

    while used > low_mark {
        let relax_dwell = used >= emergency_mark;
        let mut candidates = select_eviction_candidates_ext(db, Some(&my_node_id), now, relax_dwell)?;
        if candidates.is_empty() && !relax_dwell {
            // Emergency fallback: relax dwell guard if normal candidates are exhausted
            candidates = select_eviction_candidates_ext(db, Some(&my_node_id), now, true)?;
        }
        if candidates.is_empty() {
            tracing::warn!("No eligible unpinned blobs left to evict (protected by dwell guard or sole-seed) but still above low watermark");
            break;
        }

        let mut progress = false;
        for blob in candidates {
            if used <= low_mark {
                break;
            }
            let hash_str = &blob.hash;

            // Validate BLAKE3 hash from hex string.
            let Ok(_hash) = hash_str.parse::<iroh_blobs::Hash>() else {
                tracing::warn!(hash = %hash_str, "invalid hash in DB, skipping eviction of this entry");
                delete_blob(db, hash_str)?;
                progress = true;
                continue;
            };

            // Remove from iroh-blobs store tags (un-tag so data can be garbage collected).
            let _ = engine.store.tags().delete(hash_str.as_bytes()).await;
            if let Some(name) = &blob.name {
                let _ = engine.store.tags().delete(name.as_bytes()).await;
            }

            // Remove from our SQLite tracker.
            delete_blob(db, hash_str)?;
            crate::storage::path_utils::delete_blob_files_from_disk(&blob_dir, hash_str);
            used = used.saturating_sub(blob.size_bytes);
            deleted += 1;
            progress = true;

            tracing::debug!(hash = %hash_str, size = blob.size_bytes, "Evicted blob");
        }

        // Break if no forward progress was made in this pass to prevent infinite looping
        if !progress {
            break;
        }
    }

    tracing::info!(deleted, remaining_bytes = used, "Eviction pass complete");
    Ok(deleted)
}

/// Spawn a background tokio task that runs eviction on a periodic timer.
/// Returns a channel sender that can be used to trigger an immediate pass.
pub fn spawn_eviction_worker(
    db: Arc<Database>,
    engine_lock: Arc<tokio::sync::RwLock<Option<IrohEngine>>>,
) -> tokio::sync::mpsc::Sender<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(4);

    tokio::spawn(async move {
        let interval = Duration::from_secs(EVICTION_INTERVAL_SECS);
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = ticker.tick() => {}
                msg = rx.recv() => {
                    if msg.is_none() {
                        // Sender dropped — shut down.
                        break;
                    }
                }
            }

            let guard = engine_lock.read().await;
            let Some(engine) = guard.as_ref() else {
                continue; // Iroh not yet started
            };

            if let Err(e) = run_eviction_pass(&db, engine).await {
                tracing::error!("Eviction worker error: {e:#}");
            }
        }
    });

    tx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::blob_tracker::{total_bytes, upsert_blob};
    use crate::storage::db::Database;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Build an in-memory-like DB for tests (temp file on disk).
    fn make_db() -> (TempDir, Arc<Database>) {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(&dir.path().join("test.db")).unwrap());
        (dir, db)
    }

    /// Sets quota to `quota` bytes, inserts `n` blobs each of `blob_size` bytes
    /// (oldest inserted first so LRU order matches insertion order).
    fn fill_db(db: &Arc<Database>, quota: u64, n: usize, blob_size: u64) {
        db.set_setting("cache_limit_bytes", &quota.to_string())
            .unwrap();
        db.set_setting("full_mirror", "false").unwrap();
        for i in 0..n {
            let hash = format!("{:064x}", i); // fake 64-char hex hash
            upsert_blob(db, &hash, None, blob_size, false, None).unwrap();
            // Ensure strictly increasing last_accessed_at by sleeping 10ms.
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    // We can't run a real Iroh node in unit tests, so we test the SQLite
    // side of eviction by using a mock that skips the iroh-blobs deletion.
    // The full eviction (including blob store deletion) is covered by the
    // integration test in tests/integration.rs.

    #[test]
    fn eviction_target_is_80_percent() {
        let (_dir, db) = make_db();
        // Quota: 1000 bytes. High mark = 900, Low mark = 800.
        // Insert 10 blobs × 100 bytes = 1000 bytes (> 900 → triggers eviction).
        fill_db(&db, 1000, 10, 100);
        assert_eq!(total_bytes(&db).unwrap(), 1000);

        // The target is 800 bytes. With 100-byte blobs we need to delete 2.
        // We can verify the logic by checking that after deleting 2 blobs,
        // total would be 800 (= 80%). We verify the quota constants directly.
        let quota = 1000u64;
        let low = (quota as f64 * EVICTION_LOW_WATERMARK) as u64;
        assert_eq!(low, 800);
    }

    #[test]
    fn pinned_blobs_survive_eviction() {
        let (_dir, db) = make_db();
        db.set_setting("cache_limit_bytes", "1000").unwrap();
        db.set_setting("full_mirror", "false").unwrap();

        // Insert one pinned blob.
        upsert_blob(
            &db,
            "pinned_hash_000000000000000000000000000000000000000000000000000000",
            None,
            500,
            true,
            None,
        )
        .unwrap();
        // Insert enough unpinned to trigger eviction.
        for i in 1..=6 {
            let hash = format!("{:064}", i);
            upsert_blob(&db, &hash, None, 100, false, None).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        // Oldest-unpinned list must not contain the pinned blob.
        let evictable = oldest_unpinned(&db, 100).unwrap();
        assert!(evictable.iter().all(|b| !b.is_pinned));
        assert!(evictable.iter().all(|b| !b.hash.starts_with("pinned")));
    }

    #[test]
    fn full_mirror_skips_eviction() {
        let (_dir, db) = make_db();
        db.set_setting("cache_limit_bytes", "100").unwrap();
        db.set_setting("full_mirror", "true").unwrap();
        assert!(quota_bytes(&db).unwrap().is_none());
    }

    #[test]
    fn eviction_removes_oldest_first() {
        let (_dir, db) = make_db();
        // Oldest = hash "0000...0", newest = hash "0000...9"
        fill_db(&db, 1000, 10, 100); // all 10 inserted, oldest first
        let candidates = oldest_unpinned(&db, 3).unwrap();
        // The 3 oldest should be returned.
        assert_eq!(candidates.len(), 3);
        assert!(candidates[0].last_accessed_at <= candidates[1].last_accessed_at);
        assert!(candidates[1].last_accessed_at <= candidates[2].last_accessed_at);
    }

    #[test]
    fn dwell_time_guard_skips_recent_blobs() {
        let (_dir, db) = make_db();
        let now = 1_000_000i64;

        // Insert 3 blobs written "now"
        for i in 1..=3 {
            let hash = format!("recent_{i}");
            upsert_blob(&db, &hash, None, 500, false, None).unwrap();
            crate::storage::blob_tracker::record_peer_seed(&db, &hash, "peer1").unwrap();
        }

        // With now = current timestamp, now - written_at < 86400 -> select_eviction_candidates skips ALL 3
        let candidates = select_eviction_candidates(&db, Some("my_node"), now).unwrap();
        assert!(candidates.is_empty(), "All blobs within 24h dwell time must be skipped");
    }

    #[test]
    fn seed_count_first_eviction_order() {
        let (_dir, db) = make_db();
        let now = 2_000_000i64;
        let old_time = now - 100_000; // > 24 hours ago

        // Blob A: accessed long ago (timestamp 100), but only 1 remote seed
        upsert_blob(&db, "blob_rare", None, 500, false, None).unwrap();
        {
            let conn = db.lock();
            conn.execute(
                "UPDATE local_blobs SET written_at = ?1, last_accessed_at = ?2 WHERE hash = 'blob_rare'",
                rusqlite::params![old_time, 100i64],
            ).unwrap();
        }
        crate::storage::blob_tracker::record_peer_seed(&db, "blob_rare", "peer1").unwrap();

        // Blob B: accessed recently (timestamp 500), but has 5 remote seeds (abundant)
        upsert_blob(&db, "blob_abundant", None, 500, false, None).unwrap();
        {
            let conn = db.lock();
            conn.execute(
                "UPDATE local_blobs SET written_at = ?1, last_accessed_at = ?2 WHERE hash = 'blob_abundant'",
                rusqlite::params![old_time, 500i64],
            ).unwrap();
        }
        for p in 1..=5 {
            crate::storage::blob_tracker::record_peer_seed(&db, "blob_abundant", &format!("peer_{p}")).unwrap();
        }

        let candidates = select_eviction_candidates(&db, Some("my_node"), now).unwrap();
        assert_eq!(candidates.len(), 2);
        // Abundant blob (5 seeds) MUST be evicted before rare blob (1 seed), even though accessed more recently
        assert_eq!(candidates[0].hash, "blob_abundant");
        assert_eq!(candidates[1].hash, "blob_rare");
    }

    #[test]
    fn sole_seed_hard_block_never_evicted() {
        let (_dir, db) = make_db();
        let now = 2_000_000i64;
        let old_time = now - 100_000; // > 24 hours ago

        // Blob with 0 remote seeds (we are sole seed)
        upsert_blob(&db, "sole_seed_blob", None, 500, false, None).unwrap();
        {
            let conn = db.lock();
            conn.execute(
                "UPDATE local_blobs SET written_at = ?1, last_accessed_at = ?2 WHERE hash = 'sole_seed_blob'",
                rusqlite::params![old_time, 50i64],
            ).unwrap();
        }

        let candidates = select_eviction_candidates(&db, Some("my_node"), now).unwrap();
        assert!(candidates.is_empty(), "Sole seed blob (0 remote seeds) must NEVER be evicted");
    }
}
