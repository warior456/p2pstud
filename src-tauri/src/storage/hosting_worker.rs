// src-tauri/src/storage/hosting_worker.rs
// Cooperative Host-to-Play Cache Auto-Fill background worker.
// Actively downloads and hosts workspace files up to ~80% of the cache quota
// so that course materials are redundantly seeded across the P2P campus swarm.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use iroh::endpoint::NodeAddr;
use iroh_blobs::store::Map;
use tokio::sync::RwLock;

use crate::p2p::bandwidth::BandwidthTracker;
use crate::p2p::engine::IrohEngine;
use crate::storage::blob_tracker::{get_seed_count, list_all_blobs, touch_blob, BlobRecord};
use crate::storage::db::Database;

/// Interval between cache fill evaluations.
const HOSTING_CHECK_INTERVAL_SECS: u64 = 20;

/// Target fill fraction (80% of cache quota).
const TARGET_FILL_RATIO: f64 = 0.80;

/// Check if a blob is already completely present in the local blob store.
pub async fn is_blob_cached(store: &iroh_blobs::store::fs::Store, hash: &iroh_blobs::Hash) -> bool {
    match store.get(hash).await {
        Ok(Some(entry)) => entry.is_complete(),
        _ => false,
    }
}

/// Calculate current local storage quota and target fill size in bytes.
pub fn get_target_hosting_bytes(db: &Database) -> Result<u64> {
    let full_mirror = db
        .get_setting("full_mirror")?
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");

    if full_mirror {
        return Ok(u64::MAX);
    }

    let quota: u64 = db
        .get_setting("cache_limit_bytes")?
        .unwrap_or_else(|| "5368709120".to_string())
        .parse()
        .unwrap_or(5_368_709_120); // 5 GB default

    Ok((quota as f64 * TARGET_FILL_RATIO) as u64)
}

/// Perform one auto-fill pass. Downloads un-cached workspace files from connected peers
/// until the local cache reaches ~80% of quota.
pub async fn run_hosting_fill_pass(
    db: &Arc<Database>,
    engine: &IrohEngine,
    bandwidth: Option<&Arc<BandwidthTracker>>,
) -> Result<usize> {
    let target_bytes = get_target_hosting_bytes(db)?;
    let all_records = list_all_blobs(db)?;

    // Calculate current locally cached bytes
    let mut current_cached_bytes = 0u64;
    let mut missing_blobs = Vec::new();

    for record in &all_records {
        if let Ok(parsed_hash) = record.hash.parse::<iroh_blobs::Hash>() {
            if is_blob_cached(&engine.store, &parsed_hash).await {
                current_cached_bytes = current_cached_bytes.saturating_add(record.size_bytes);
            } else {
                missing_blobs.push(record.clone());
            }
        }
    }

    // If cache is already at or above target quota (80% or 100% in mirror), nothing to fetch
    if current_cached_bytes >= target_bytes || missing_blobs.is_empty() {
        return Ok(0);
    }

    let remotes: Vec<_> = engine.endpoint.remote_info_iter().collect();
    if remotes.is_empty() {
        return Ok(0); // No peers currently reachable to fetch from
    }

    // Smart Cache Prioritization: Sort missing files by least seeds first!
    // Rare/endangered files with fewest seeders across the swarm are replicated first,
    // ensuring files with only 1 seed never disappear from the campus network.
    let mut missing_with_seeds: Vec<(BlobRecord, usize)> = Vec::with_capacity(missing_blobs.len());
    for missing in missing_blobs {
        let seeds = get_seed_count(db, &missing.hash, false).unwrap_or(1);
        missing_with_seeds.push((missing, seeds));
    }

    missing_with_seeds.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| a.0.size_bytes.cmp(&b.0.size_bytes))
    });

    let mut downloaded_count = 0;

    for (missing, seeds) in missing_with_seeds {
        if current_cached_bytes >= target_bytes {
            break;
        }

        let parsed_hash = match missing.hash.parse::<iroh_blobs::Hash>() {
            Ok(h) => h,
            Err(_) => continue,
        };

        let mut success = false;

        // Try owner first
        if let Some(owner) = &missing.owner_node_id {
            if let Ok(owner_id) = owner.parse::<iroh::NodeId>() {
                let addr = NodeAddr::new(owner_id);
                if let Ok(progress) = engine.blobs.clone().client().download(parsed_hash, addr).await {
                    let _ = progress.finish().await;
                    if is_blob_cached(&engine.store, &parsed_hash).await {
                        success = true;
                    }
                }
            }
        }

        // Fallback to any connected swarm peer
        if !success {
            for remote in &remotes {
                let addr = NodeAddr::new(remote.node_id);
                if let Ok(progress) = engine.blobs.clone().client().download(parsed_hash, addr).await {
                    let _ = progress.finish().await;
                    if is_blob_cached(&engine.store, &parsed_hash).await {
                        success = true;
                        break;
                    }
                }
            }
        }

        if success {
            current_cached_bytes = current_cached_bytes.saturating_add(missing.size_bytes);
            downloaded_count += 1;
            let _ = touch_blob(db, &missing.hash);
            if let Some(bw) = bandwidth {
                bw.record_download(missing.size_bytes);
            }
            tracing::info!(
                hash = %missing.hash,
                name = ?missing.name,
                size = missing.size_bytes,
                seeds = seeds,
                "Pre-fetched rare workspace file (least seeds first) to actively host in P2P swarm"
            );
        }
    }

    Ok(downloaded_count)
}

/// Background worker loop that periodically keeps the cache filled to ~80% for swarm hosting.
pub fn spawn_hosting_worker(
    db: Arc<Database>,
    engine_lock: Arc<RwLock<Option<IrohEngine>>>,
    bandwidth: Arc<BandwidthTracker>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(HOSTING_CHECK_INTERVAL_SECS));
        loop {
            interval.tick().await;

            let guard = engine_lock.read().await;
            if let Some(engine) = guard.as_ref() {
                if let Err(e) = run_hosting_fill_pass(&db, engine, Some(&bandwidth)).await {
                    tracing::debug!("Hosting worker pass note: {:#}", e);
                }
            }
        }
    });
}
