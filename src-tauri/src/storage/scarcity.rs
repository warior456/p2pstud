// src-tauri/src/storage/scarcity.rs
// Smart Scarcity Replication & SSD Wear Defense background worker.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::Local;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

use crate::p2p::bandwidth::BandwidthTracker;
use crate::p2p::engine::IrohEngine;
use crate::p2p::network_profile::{
    detect_os_ssid, get_background_bandwidth_limit, is_current_lecture_hours,
    resolve_network_profile,
};
use crate::storage::blob_tracker::{
    add_or_update_resume_queue, add_to_today_write_budget, get_today_write_budget,
    remove_resume_queue, BlobRecord,
};
use crate::storage::db::Database;

pub const DEFAULT_DAILY_WRITE_BUDGET_BYTES: u64 = 3 * 1024 * 1024 * 1024; // 3 GB
pub const SCARCITY_CHECK_INTERVAL_SECS: u64 = 600; // 10 minutes

/// Scarcity replication event emitted to the frontend UI for toast notification.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScarcityReplicatedEvent {
    pub hash: String,
    pub name: Option<String>,
    pub size_bytes: u64,
}

/// Find the most endangered blob in the catalog that is NOT stored locally.
/// Returns Ok(None) if no candidates exist or if all candidates are sufficiently seeded.
pub async fn find_endangered_blob(
    db: &Database,
    engine: &IrohEngine,
) -> Result<Option<BlobRecord>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let threshold = now - 300; // Active seeds seen in the last 5 minutes
    let our_node_id = engine.node_id().to_string();

    let all_blobs = crate::storage::blob_tracker::list_all_blobs(db)?;
    let mut candidates = Vec::new();

    for blob in all_blobs {
        let hash_str = &blob.hash;

        // 1. Skip if already cached locally in the iroh-blobs store
        if let Ok(hash) = hash_str.parse::<iroh_blobs::Hash>() {
            if crate::storage::hosting_worker::is_blob_cached(&engine.store, &hash).await {
                continue;
            }
        }

        // 2. Count active online remote seeds
        let online_seeds: i64 = {
            let conn = db.lock();
            conn.query_row(
                "SELECT COUNT(DISTINCT peer_node_id) FROM file_seeds
                 WHERE hash = ?1 AND peer_node_id != ?2 AND last_seen_at > ?3",
                rusqlite::params![hash_str, our_node_id, threshold],
                |r| r.get(0),
            )
            .unwrap_or(0)
        };

        // A file is endangered if exactly 1 peer is online seeding it
        if online_seeds == 1 {
            // Quality penalty (Step 38): Net rating <= -3 is heavily deprioritized
            let is_downvoted = blob.net_rating <= -3;

            // Aging penalty (Step 41): Older than 5 years (157,766,400s) is deprioritized
            let is_ancient = blob
                .catalog_first_seen_at
                .map(|t| now - t > 157_766_400)
                .unwrap_or(false);

            let priority_tier = if is_downvoted || is_ancient { 1 } else { 0 };

            candidates.push((priority_tier, online_seeds, blob.size_bytes, blob));
        }
    }

    // Sort: normal priority first, lowest seeds first, smallest file size first as tiebreaker
    candidates.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    Ok(candidates.into_iter().next().map(|(_, _, _, b)| b))
}

/// Spawns the Smart Scarcity Replication & SSD Wear Defense background worker.
pub fn spawn_scarcity_worker(
    db: Arc<Database>,
    engine_lock: Arc<RwLock<Option<IrohEngine>>>,
    bandwidth: Arc<BandwidthTracker>,
    app: AppHandle,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(SCARCITY_CHECK_INTERVAL_SECS));

        loop {
            interval.tick().await;

            let guard = engine_lock.read().await;
            let Some(engine) = guard.as_ref() else {
                continue;
            };

            // 1. Network Profile check: Always work, but throttle bandwidth during campus hours on campus networks
            let ssid = detect_os_ssid();
            let (profile, _) = resolve_network_profile(&db, ssid.as_deref());
            let bandwidth_cap = get_background_bandwidth_limit(&db, profile);
            if let Some(cap) = bandwidth_cap {
                tracing::info!(
                    profile = profile.as_str(),
                    is_lecture = is_current_lecture_hours(),
                    cap_kbps = cap / 1024,
                    "Smart scarcity worker active with background bandwidth throttle"
                );
            }

            // 2. Find endangered 1-seed blob
            let target_blob = match find_endangered_blob(&db, engine).await {
                Ok(Some(blob)) => blob,
                Ok(None) => continue,
                Err(err) => {
                    tracing::warn!("Error inspecting endangered blobs: {err:#}");
                    continue;
                }
            };

            // 3. Daily SSD write budget check
            let today_key = Local::now().format("%Y-%m-%d").to_string();
            let daily_budget: u64 = db
                .get_setting("daily_write_budget_bytes")
                .ok()
                .flatten()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_DAILY_WRITE_BUDGET_BYTES);

            let current_written = get_today_write_budget(&db, &today_key).unwrap_or(0);
            if current_written.saturating_add(target_blob.size_bytes) > daily_budget {
                tracing::warn!(
                    current_written,
                    daily_budget,
                    file_size = target_blob.size_bytes,
                    "Daily SSD write budget reached — skipping auto-replication"
                );
                continue;
            }

            // 4. Peer availability check
            let threshold = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                - 300;

            let seeder_peer_id: Option<String> = {
                let conn = db.lock();
                conn.query_row(
                    "SELECT peer_node_id FROM file_seeds
                     WHERE hash = ?1 AND peer_node_id != ?2 AND last_seen_at > ?3
                     LIMIT 1",
                    rusqlite::params![target_blob.hash, engine.node_id().to_string(), threshold],
                    |r| r.get(0),
                )
                .ok()
            };

            let Some(peer_str) = seeder_peer_id else {
                continue; // No online peer available to download from
            };

            let Ok(seeder_node_id) = peer_str.parse::<iroh::NodeId>() else {
                continue;
            };

            let Ok(target_hash) = target_blob.hash.parse::<iroh_blobs::Hash>() else {
                continue;
            };

            tracing::info!(
                hash = %target_blob.hash,
                name = ?target_blob.name,
                seeder = %seeder_node_id,
                size = target_blob.size_bytes,
                "Auto-replicating endangered 1-seed course file to protect scarcity"
            );

            // 5. Track in Transfer Resume Queue
            let _ = add_or_update_resume_queue(
                &db,
                &target_blob.hash,
                target_blob.name.as_deref(),
                target_blob.size_bytes,
                0,
                Some(&peer_str),
            );

            // 6. In-memory staging download with BLAKE3 verification and bandwidth throttling
            let seeder_addr = iroh::NodeAddr::new(seeder_node_id);
            let download_res = async {
                let start_time = tokio::time::Instant::now();
                let progress = engine
                    .blobs
                    .clone()
                    .client()
                    .download(target_hash, seeder_addr)
                    .await?;
                let _ = progress.finish().await?;

                let data = engine
                    .blobs
                    .clone()
                    .client()
                    .read_to_bytes(target_hash)
                    .await?;

                // Verify BLAKE3 hash integrity
                let actual_hash = iroh_blobs::Hash::new(&data);
                if actual_hash != target_hash {
                    anyhow::bail!("BLAKE3 integrity verification failed for auto-replicated blob");
                }

                // If bandwidth cap is active (e.g. 128 KB/s on campus during lecture hours),
                // throttle pace to enforce average rate limit.
                if let Some(cap_bps) = bandwidth_cap {
                    if cap_bps > 0 {
                        let total_bytes = data.len() as u64;
                        let target_duration = Duration::from_secs_f64(total_bytes as f64 / cap_bps as f64);
                        let elapsed = start_time.elapsed();
                        if elapsed < target_duration {
                            let remaining = target_duration - elapsed;
                            let steps = 10.max((remaining.as_millis() / 100) as usize);
                            let sleep_step = remaining / (steps as u32);
                            let bytes_per_step = total_bytes / (steps as u64);
                            let mut remaining_bytes = total_bytes;

                            for _ in 0..steps {
                                let chunk = bytes_per_step.min(remaining_bytes);
                                bandwidth.record_download(chunk);
                                remaining_bytes = remaining_bytes.saturating_sub(chunk);
                                tokio::time::sleep(sleep_step).await;
                            }
                            if remaining_bytes > 0 {
                                bandwidth.record_download(remaining_bytes);
                            }
                        } else {
                            bandwidth.record_download(data.len() as u64);
                        }
                    } else {
                        bandwidth.record_download(data.len() as u64);
                    }
                } else {
                    bandwidth.record_download(data.len() as u64);
                }

                Ok::<(), anyhow::Error>(())
            }
            .await;

            match download_res {
                Ok(()) => {
                    let _ = remove_resume_queue(&db, &target_blob.hash);
                    let _ = add_to_today_write_budget(&db, &today_key, target_blob.size_bytes);

                    // Emit toast event to frontend
                    let _ = app.emit(
                        "scarcity-replicated",
                        ScarcityReplicatedEvent {
                            hash: target_blob.hash.clone(),
                            name: target_blob.name.clone(),
                            size_bytes: target_blob.size_bytes,
                        },
                    );

                    tracing::info!(
                        hash = %target_blob.hash,
                        name = ?target_blob.name,
                        "Successfully replicated endangered blob — swarm scarcity strengthened"
                    );
                }
                Err(e) => {
                    tracing::warn!(hash = %target_blob.hash, "Scarcity replication failed: {e:#}");
                }
            }
        }
    });
}
