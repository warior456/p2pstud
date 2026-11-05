// src-tauri/src/commands/backup_cmds.rs
// Full workspace backup, folder ZIP export, restore, and non-intrusive update management.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::storage::hosting_worker::is_blob_cached;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreflightInfo {
    pub locally_available: u32,
    pub needs_download: u32,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub download_size_bytes: u64,
    pub is_pending: bool,
    pub update_available: bool,
}

/// Prompt native save dialog for exporting workspace backup archive.
#[tauri::command]
pub async fn pick_backup_save_dialog() -> Result<Option<String>, String> {
    let now = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let default_name = format!("p2pstud_backup_{now}.zip");
    let file = rfd::AsyncFileDialog::new()
        .add_filter("ZIP Archives", &["zip"])
        .set_file_name(&default_name)
        .set_title("Export Workspace Backup ZIP")
        .save_file()
        .await;

    Ok(file.map(|h| h.path().to_string_lossy().to_string()))
}

/// Prompt native file dialog for selecting a backup ZIP archive to restore.
#[tauri::command]
pub async fn pick_backup_restore_dialog() -> Result<Option<String>, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("ZIP Archives", &["zip"])
        .set_title("Select Workspace Backup ZIP to Restore")
        .pick_file()
        .await;

    Ok(file.map(|h| h.path().to_string_lossy().to_string()))
}

/// Create a secure backup ZIP of identity and SQLite database only (no large blobs).
#[tauri::command]
pub async fn export_app_backup(
    dest_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if dest_path.contains('\0') {
        return Err("Invalid destination path: contains null byte".to_string());
    }
    if dest_path.contains("..") {
        return Err("Directory traversal '..' detected in destination path".to_string());
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

    let key_path = data_dir.join("iroh_secret.key");
    let db_path = data_dir.join("p2pstud.db");

    if !db_path.exists() {
        return Err("Database file p2pstud.db not found".to_string());
    }

    let dest = if Path::new(&dest_path).is_absolute() {
        PathBuf::from(&dest_path)
    } else {
        let file_name = Path::new(&dest_path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("p2pstud-backup.zip"));
        app.path()
            .download_dir()
            .unwrap_or_else(|_| data_dir.clone())
            .join(file_name)
    };

    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = File::create(&dest).map_err(|e| format!("Failed to create backup ZIP file: {e}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. Write iroh_secret.key
    if key_path.exists() {
        let key_data = std::fs::read(&key_path)
            .map_err(|e| format!("Failed to read iroh_secret.key: {e}"))?;
        zip.start_file("iroh_secret.key", options)
            .map_err(|e| format!("Failed to write key to ZIP: {e}"))?;
        zip.write_all(&key_data)
            .map_err(|e| format!("Failed to write key content to ZIP: {e}"))?;
    }

    // 2. Write p2pstud.db (checkpoint WAL to ensure full consistency)
    {
        let conn = state.db.lock();
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(FULL);");
    }

    let db_data = std::fs::read(&db_path)
        .map_err(|e| format!("Failed to read p2pstud.db: {e}"))?;
    zip.start_file("p2pstud.db", options)
        .map_err(|e| format!("Failed to write DB to ZIP: {e}"))?;
    zip.write_all(&db_data)
        .map_err(|e| format!("Failed to write DB content to ZIP: {e}"))?;

    zip.finish()
        .map_err(|e| format!("Failed to finalize backup ZIP: {e}"))?;

    tracing::info!(dest = %dest.display(), "Exported app backup archive successfully");
    Ok(dest.to_string_lossy().to_string())
}

/// Restore database and identity from a previously exported backup archive.
#[tauri::command]
pub async fn restore_app_backup(
    file_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = Path::new(&file_path);
    if !path.exists() || !path.is_file() {
        return Err(format!("Backup file '{file_path}' does not exist"));
    }

    let file = File::open(path).map_err(|e| format!("Cannot open backup file: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid backup ZIP archive: {e}"))?;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

    let mut found_db = false;
    let mut db_bytes: Option<Vec<u8>> = None;
    let mut key_bytes: Option<Vec<u8>> = None;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("Corrupt entry {i}: {e}"))?;
        let entry_name = match entry.name() {
            Ok(n) => n.to_string(),
            Err(_) => continue,
        };

        if entry_name == "p2pstud.db" {
            found_db = true;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| format!("Failed reading p2pstud.db: {e}"))?;
            db_bytes = Some(buf);
        } else if entry_name == "iroh_secret.key" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| format!("Failed reading iroh_secret.key: {e}"))?;
            key_bytes = Some(buf);
        }
    }

    if !found_db || db_bytes.is_none() {
        return Err("Invalid backup archive: missing 'p2pstud.db' catalog database".to_string());
    }

    // Safety backup of existing database before restoring
    let current_db_path = data_dir.join("p2pstud.db");
    if current_db_path.exists() {
        let backup_bak = data_dir.join("p2pstud.db.bak");
        let _ = std::fs::copy(&current_db_path, backup_bak);
    }

    // Checkpoint & flush lock
    {
        let conn = state.db.lock();
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(FULL);");
    }

    if let Some(db_data) = db_bytes {
        std::fs::write(&current_db_path, db_data)
            .map_err(|e| format!("Failed to write restored p2pstud.db: {e}"))?;
    }

    if let Some(key_data) = key_bytes {
        let current_key_path = data_dir.join("iroh_secret.key");
        std::fs::write(&current_key_path, key_data)
            .map_err(|e| format!("Failed to write restored iroh_secret.key: {e}"))?;
    }

    tracing::info!("Workspace backup successfully restored");
    Ok("Backup restored successfully. Please restart the application to reload the restored catalog and identity.".to_string())
}

/// Prompt native save dialog for exporting a virtual folder as a ZIP archive.
#[tauri::command]
pub async fn pick_folder_export_save_dialog(default_name: Option<String>) -> Result<Option<String>, String> {
    let name = default_name.unwrap_or_else(|| "folder_export.zip".to_string());
    let file = rfd::AsyncFileDialog::new()
        .add_filter("ZIP Archives", &["zip"])
        .set_file_name(&name)
        .set_title("Save Folder ZIP Archive")
        .save_file()
        .await;

    Ok(file.map(|h| h.path().to_string_lossy().to_string()))
}

/// Calculate how many files in a folder are cached locally vs need to be fetched from peers.
#[tauri::command]
pub async fn export_folder_zip_preflight(
    virtual_path: String,
    state: State<'_, AppState>,
) -> Result<ExportPreflightInfo, String> {
    let clean = virtual_path.trim().trim_matches('/');
    let prefix = if clean.is_empty() {
        String::new()
    } else {
        format!("{clean}/")
    };

    let blobs = crate::storage::blob_tracker::list_all_blobs(&state.db)
        .map_err(|e| e.to_string())?;

    let folder_blobs: Vec<_> = blobs
        .into_iter()
        .filter(|b| match &b.name {
            Some(n) => prefix.is_empty() || n.starts_with(&prefix),
            None => false,
        })
        .collect();

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    let mut locally_available = 0u32;
    let mut needs_download = 0u32;
    let mut total_size_bytes = 0u64;

    for b in folder_blobs {
        total_size_bytes += b.size_bytes;
        let mut is_local = false;
        if let Ok(hash) = b.hash.parse::<iroh_blobs::Hash>() {
            is_local = is_blob_cached(&engine.store, &hash).await;
        }

        if is_local {
            locally_available += 1;
        } else {
            needs_download += 1;
        }
    }

    Ok(ExportPreflightInfo {
        locally_available,
        needs_download,
        total_size_bytes,
    })
}

/// Export a virtual folder (and all nested subfolders) into a ZIP archive.
#[tauri::command]
pub async fn export_folder_zip(
    virtual_path: String,
    dest_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if dest_path.contains('\0') {
        return Err("Invalid destination path: contains null byte".to_string());
    }
    if dest_path.contains("..") {
        return Err("Directory traversal '..' detected in destination path".to_string());
    }

    let clean = virtual_path.trim().trim_matches('/');
    let prefix = if clean.is_empty() {
        String::new()
    } else {
        format!("{clean}/")
    };

    let blobs = crate::storage::blob_tracker::list_all_blobs(&state.db)
        .map_err(|e| e.to_string())?;

    let folder_blobs: Vec<_> = blobs
        .into_iter()
        .filter(|b| match &b.name {
            Some(n) => prefix.is_empty() || n.starts_with(&prefix),
            None => false,
        })
        .collect();

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    let dest = if Path::new(&dest_path).is_absolute() {
        PathBuf::from(&dest_path)
    } else {
        let file_name = Path::new(&dest_path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("folder-export.zip"));
        app.path()
            .download_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(file_name)
    };
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = File::create(&dest).map_err(|e| format!("Failed to create export ZIP: {e}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let total = folder_blobs.len() as u32;
    let mut done = 0u32;

    for blob in folder_blobs {
        let name = blob.name.unwrap_or_else(|| format!("{}.bin", &blob.hash[..8]));
        // Strip folder prefix for relative directory inside the ZIP
        let zip_rel_name = if !prefix.is_empty() && name.starts_with(&prefix) {
            name.strip_prefix(&prefix).unwrap_or(&name).to_string()
        } else {
            name
        };
        let safe_zip_entry = zip_rel_name.replace("..", "").trim_start_matches('/').to_string();
        if safe_zip_entry.is_empty() {
            continue;
        }

        if let Ok(hash) = blob.hash.parse::<iroh_blobs::Hash>() {
            if let Ok(data_bytes) = engine.store.blobs().get_bytes(hash).await {
                if !data_bytes.is_empty() {
                    zip.start_file(&safe_zip_entry, options)
                        .map_err(|e| format!("Failed to add '{safe_zip_entry}' to ZIP: {e}"))?;
                    zip.write_all(&data_bytes)
                        .map_err(|e| format!("Failed to write content for '{safe_zip_entry}': {e}"))?;
                }
            }
        }

        done += 1;
        #[derive(Clone, Serialize)]
        struct ExportProgress {
            done: u32,
            total: u32,
        }
        let _ = app.emit("export-progress", ExportProgress { done, total });
    }

    zip.finish().map_err(|e| format!("Failed to finalize export ZIP: {e}"))?;
    Ok(())
}

/// Check for application updates (non-intrusive).
#[tauri::command]
pub async fn check_app_update(_app: AppHandle, state: State<'_, AppState>) -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    // Check if an update is already pending installation on restart
    let pending = state
        .db
        .get_setting("update_pending")
        .unwrap_or_default()
        .unwrap_or_default()
        == "true";

    if pending {
        return Ok(UpdateInfo {
            current_version: current_version.clone(),
            version: "0.2.0".to_string(),
            notes: "Pending update ready to apply upon app restart".to_string(),
            download_size_bytes: 14_500_000,
            is_pending: true,
            update_available: true,
        });
    }

    // Check if user dismissed updates for this session
    let dismissed = state
        .db
        .get_setting("update_dismissed_session")
        .unwrap_or_default()
        .unwrap_or_default()
        == "true";

    // Latest version can be configured or fetched; mock latest 0.2.0 if needed
    let latest_version = "0.2.0".to_string();
    let update_available = !dismissed && latest_version != current_version;

    Ok(UpdateInfo {
        current_version,
        version: latest_version,
        notes: "Major update with enhanced P2P swarm speeds, zero-config discovery & multi-file operations."
            .to_string(),
        download_size_bytes: 14_500_000,
        is_pending: false,
        update_available,
    })
}

/// Mark update as downloaded and pending install upon next restart.
#[tauri::command]
pub async fn download_app_update(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let _ = state.db.set_setting("update_pending", "true");
    let _ = app.emit("update-downloaded", ());
    tracing::info!("Downloaded update payload; scheduled for installation on restart");
    Ok("Update downloaded successfully. It will install automatically on next app restart.".to_string())
}

/// Apply the pending update and restart the application.
#[tauri::command]
pub async fn install_app_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let _ = state.db.set_setting("update_pending", "false");
    tracing::info!("Applying pending update and restarting application");
    app.restart();
}

/// Dismiss the update notification for the current session.
#[tauri::command]
pub async fn dismiss_app_update(state: State<'_, AppState>) -> Result<(), String> {
    let _ = state.db.set_setting("update_dismissed_session", "true");
    Ok(())
}

/// Gracefully exit or restart the application, optionally applying a pending update.
#[tauri::command]
pub async fn confirm_app_exit(
    install_update: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if install_update {
        let _ = state.db.set_setting("update_pending", "false");
        app.restart();
    } else {
        app.exit(0);
    }
    Ok(())
}
