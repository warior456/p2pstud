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
    /// Instantaneous outbound network transfer rate (bytes per second).
    pub upload_bytes_per_sec: u64,
    /// Instantaneous inbound network transfer rate (bytes per second).
    pub download_bytes_per_sec: u64,
}

/// Global application settings.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub relay_url: String,
    pub cache_limit_bytes: u64,
    pub full_mirror: bool,
    pub node_id: Option<String>,
    pub network_secret: String,
    #[serde(default)]
    pub onboarding_complete: Option<String>,
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

    let (upload_bytes_per_sec, download_bytes_per_sec) = state.bandwidth.get_rates();

    Ok(StorageStats {
        used_bytes,
        quota_bytes,
        pinned_bytes,
        peer_count,
        full_mirror,
        upload_bytes_per_sec,
        download_bytes_per_sec,
    })
}

/// Return all application configuration settings.
#[tauri::command]
pub async fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let relay_url = state
        .db
        .get_setting("relay_url")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let cache_limit_bytes: u64 = state
        .db
        .get_setting("cache_limit_bytes")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "5368709120".to_string())
        .parse()
        .unwrap_or(5_368_709_120);

    let full_mirror = state
        .db
        .get_setting("full_mirror")
        .map_err(|e| e.to_string())?
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");

    let node_id = {
        let guard = state.engine.read().await;
        guard.as_ref().map(|e| e.node_id().to_string())
    };

    let network_secret = state
        .db
        .get_setting("network_secret")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let onboarding_complete = state
        .db
        .get_setting("onboarding_complete")
        .map_err(|e| e.to_string())?;

    Ok(AppSettings {
        relay_url,
        cache_limit_bytes,
        full_mirror,
        node_id,
        network_secret,
        onboarding_complete,
    })
}

/// Save updated application settings.
#[tauri::command]
pub async fn update_app_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .set_setting("relay_url", &settings.relay_url)
        .map_err(|e| e.to_string())?;

    state
        .db
        .set_setting(
            "cache_limit_bytes",
            &settings.cache_limit_bytes.to_string(),
        )
        .map_err(|e| e.to_string())?;

    state
        .db
        .set_setting("full_mirror", &settings.full_mirror.to_string())
        .map_err(|e| e.to_string())?;

    state
        .db
        .set_setting("network_secret", settings.network_secret.trim())
        .map_err(|e| e.to_string())?;

    if let Some(ref ob) = settings.onboarding_complete {
        state
            .db
            .set_setting("onboarding_complete", ob.trim())
            .map_err(|e| e.to_string())?;
    }

    tracing::info!(?settings, "Updated application settings");
    Ok(())
}

/// Update the P2P network secret / workspace passphrase.
#[tauri::command]
pub async fn set_network_secret(
    secret: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .set_setting("network_secret", secret.trim())
        .map_err(|e| e.to_string())?;
    tracing::info!("Updated network secret / workspace passphrase");
    Ok(())
}

/// Update the cache limit quota. Accepts either `limit_bytes` or `bytes` / `full_mirror`.
#[tauri::command]
pub async fn set_cache_limit(
    limit_bytes: Option<u64>,
    bytes: Option<Option<u64>>,
    full_mirror: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(fm) = full_mirror {
        state
            .db
            .set_setting("full_mirror", &fm.to_string())
            .map_err(|e| e.to_string())?;
    }
    let target_limit = limit_bytes.or(bytes.flatten());
    if let Some(val) = target_limit {
        state
            .db
            .set_setting("cache_limit_bytes", &val.to_string())
            .map_err(|e| e.to_string())?;
        tracing::info!(val, "Updated cache limit bytes");
    }
    Ok(())
}

/// Toggle full-mirror mode.
#[tauri::command]
pub async fn set_full_mirror(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .set_setting("full_mirror", &enabled.to_string())
        .map_err(|e| e.to_string())?;
    tracing::info!(enabled, "Toggled full mirror mode");
    Ok(())
}

/// Purge all unpinned cached blobs from disk to immediately free space.
#[tauri::command]
pub async fn purge_cache(state: State<'_, AppState>) -> Result<u64, String> {
    let unpinned = oldest_unpinned(&state.db, 10_000).map_err(|e| e.to_string())?;
    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    let mut freed_bytes: u64 = 0;
    for blob in unpinned {
        if let Ok(hash) = blob.hash.parse::<iroh_blobs::Hash>() {
            let _ = engine.blobs.clone().client().delete_blob(hash).await;
            let _ = delete_blob(&state.db, &blob.hash);
            freed_bytes += blob.size_bytes;
        }
    }

    tracing::info!(freed_bytes, "Purged unpinned cache");
    Ok(freed_bytes)
}

/// Alias for purge_cache
#[tauri::command]
pub async fn purge_unpinned_cache(state: State<'_, AppState>) -> Result<u64, String> {
    purge_cache(state).await
}

/// Open the local data storage folder in the system file manager.
#[tauri::command]
pub async fn open_data_dir(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(data_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(data_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(data_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}


/// Retrieve an arbitrary application setting by key.
#[tauri::command]
pub async fn get_setting(key: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    state.db.get_setting(&key).map_err(|e| e.to_string())
}

/// Persist an arbitrary application setting by key.
#[tauri::command]
pub async fn set_setting(key: String, value: String, state: State<'_, AppState>) -> Result<(), String> {
    state.db.set_setting(&key, &value).map_err(|e| e.to_string())
}
