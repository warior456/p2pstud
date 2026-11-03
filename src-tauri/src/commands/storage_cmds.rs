// src-tauri/src/commands/storage_cmds.rs
// Tauri IPC commands for cache quota management, settings, and storage statistics.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::storage::blob_tracker::{delete_blob, oldest_unpinned, total_bytes, total_pinned_bytes};
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

/// Global application settings.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub relay_url: String,
    pub cache_limit_bytes: u64,
    pub full_mirror: bool,
    pub node_id: Option<String>,
}

/// Return current storage statistics to the frontend.
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

/// Return all application configuration settings.
#[tauri::command]
pub async fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let relay_url = state
        .db
        .get_setting("relay_url")
        .map_err(|e| e.to_string())?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://use1-1.relay.iroh.network/".to_string());

    let full_mirror = state
        .db
        .get_setting("full_mirror")
        .map_err(|e| e.to_string())?
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");

    let cache_limit_bytes: u64 = state
        .db
        .get_setting("cache_limit_bytes")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "5368709120".to_string())
        .parse()
        .unwrap_or(5_368_709_120);

    let node_id = {
        let guard = state.engine.read().await;
        guard.as_ref().map(|e| e.node_id().to_string())
    };

    Ok(AppSettings {
        relay_url,
        cache_limit_bytes,
        full_mirror,
        node_id,
    })
}

/// Update the custom relay URL in settings.
#[tauri::command]
pub async fn set_relay_url(relay_url: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .set_setting("relay_url", relay_url.trim())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Update the cache quota.
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

/// Purge all unpinned cached blobs to instantly free up disk space.
#[tauri::command]
pub async fn purge_unpinned_cache(state: State<'_, AppState>) -> Result<u64, String> {
    let unpinned = oldest_unpinned(&state.db, usize::MAX).map_err(|e| e.to_string())?;
    let mut freed_bytes = 0u64;

    let guard = state.engine.read().await;
    let engine = guard.as_ref();

    for blob in unpinned {
        let _ = delete_blob(&state.db, &blob.hash);
        if let Some(e) = engine {
            if let Ok(parsed_hash) = blob.hash.parse::<iroh_blobs::Hash>() {
                let _ = e.blobs.clone().client().delete_blob(parsed_hash).await;
            }
        }
        freed_bytes += blob.size_bytes;
    }

    Ok(freed_bytes)
}

/// Open the app data folder on disk in Windows Explorer.
#[tauri::command]
pub async fn open_data_dir(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(data_dir)
            .spawn();
    }
    Ok(())
}
