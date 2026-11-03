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

use crate::p2p::engine::IrohEngine;
use crate::storage::blob_tracker::{list_all_blobs, touch_blob};
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
pub async fn run_hosting_fill_pass(db: &Arc<Database>, engine: &IrohEngine) -> Result<usize> {
    let target_bytes = get_target_hosting_bytes(db)?;
    let all_records = list_all_blobs(db)?;

    // Calculate current locally cached bytes
    let mut current_cached_bytes = 0u64;
    let mut missing_blobs = Vec::new();

    for r in &all_records {
        if let Ok(hash) = r.hash.parse::<iroh_blobs::Hash>() {
            if is_blob_cached(&engine.store, &hash).await {
                current_cached_bytes = current_cached_bytes.saturating_add(r.size_bytes);
            } else {
                missing_blobs.push(r.clone());
            }
        }
    }

    if current_cached_bytes >= target_bytes || missing_blobs.is_empty() {
        return Ok(0);
    }

    // Collect available remote peers
    let remotes: Vec<_> = engine.endpoint.remote_info_iter().collect();
    if remotes.is_empty() {
        return Ok(0);
    }

    let mut downloaded_count = 0;

    for missing in missing_blobs {
        if current_cached_bytes.saturating_add(missing.size_bytes) > target_bytes {
            // Reached target fill level (~80%)
            break;
        }

        let Ok(parsed_hash) = missing.hash.parse::<iroh_blobs::Hash>() else {
            continue;
        };

        // Try downloading from owner or connected peers
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
            tracing::info!(
                hash = %missing.hash,
                name = ?missing.name,
                size = missing.size_bytes,
                "Pre-fetched workspace file to actively host in P2P swarm (~80% target)"
            );
        }
    }

    Ok(downloaded_count)
}

/// Background worker loop that periodically keeps the cache filled to ~80% for swarm hosting.
pub fn spawn_hosting_worker(db: Arc<Database>, engine_lock: Arc<RwLock<Option<IrohEngine>>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(HOSTING_CHECK_INTERVAL_SECS));
        loop {
            interval.tick().await;

            let guard = engine_lock.read().await;
            if let Some(engine) = guard.as_ref() {
                if let Err(e) = run_hosting_fill_pass(&db, engine).await {
                    tracing::debug!("Hosting worker pass note: {:#}", e);
                }
            }
        }
    });
}
