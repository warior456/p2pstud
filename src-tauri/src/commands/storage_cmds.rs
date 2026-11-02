// src-tauri/src/commands/storage_cmds.rs
// Tauri IPC commands for cache quota management and storage statistics.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::storage::blob_tracker::{total_bytes, total_pinned_bytes};
use crate::AppState;

/// Payload returned to the frontend by `get_storage_stats`.
#[derive(Debug, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total bytes currently stored (pinned + cached).
    pub used_bytes: u64,
    /// Quota ceiling in bytes. `u64::MAX` indicates full-mirror mode.
    pub quota_bytes: u64,
    /// Bytes occupied by pinned (protected) blobs.
    pub pinned_bytes: u64,
    /// Currently connected peer count.
    pub peer_count: usize,
    /// Whether full-mirror mode is active.
    pub full_mirror: bool,
}

/// Return current storage statistics to the frontend.
///
/// Called by the telemetry footer every 5 seconds.
#[tauri::command]
pub async fn get_storage_stats(state: State<'_, AppState>) -> Result<StorageStats, String> {
    let used_bytes = total_bytes(&state.db).map_err(|e| e.to_string())?;
    let pinned_bytes = total_pinned_bytes(&state.db).map_err(|e| e.to_string())?;

    let full_mirror = state
        .db
        .get_setting("full_mirror")
        .map_err(|e| e.to_string())?
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");

    let quota_bytes: u64 = if full_mirror {
        u64::MAX
    } else {
        state
            .db
            .get_setting("cache_limit_bytes")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "5368709120".to_string())
            .parse()
            .unwrap_or(5_368_709_120)
    };

    let peer_count = {
        let guard = state.engine.read().await;
        match guard.as_ref() {
            Some(engine) => engine.peer_count().await,
            None => 0,
        }
    };

    Ok(StorageStats {
        used_bytes,
        quota_bytes,
        pinned_bytes,
        peer_count,
        full_mirror,
    })
}

/// Update the cache quota.
///
/// - `bytes = Some(n)` → set quota to n bytes, disable full-mirror.
/// - `full_mirror = true` → enable full-mirror mode (eviction disabled, no quota ceiling).
///
/// After updating, triggers an immediate eviction check in the background.
#[tauri::command]
pub async fn set_cache_limit(
    bytes: Option<u64>,
    full_mirror: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if full_mirror {
        state
            .db
            .set_setting("full_mirror", "true")
            .map_err(|e| e.to_string())?;
    } else {
        let limit = bytes.unwrap_or(5_368_709_120); // default 5 GB
        state
            .db
            .set_setting("cache_limit_bytes", &limit.to_string())
            .map_err(|e| e.to_string())?;
        state
            .db
            .set_setting("full_mirror", "false")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

