// src-tauri/src/commands/backup_cmds.rs
// Full workspace backup, folder ZIP export, and non-intrusive update management.

use std::fs::File;
use std::io::Write;
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
    pub version: String,
    pub notes: String,
    pub download_size_bytes: u64,
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
            if is_blob_cached(&engine.store, &hash).await {
                is_local = true;
            }
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
            if let Ok(data_bytes) = engine.blobs.clone().client().read_to_bytes(hash).await {
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
pub async fn check_app_update(_app: AppHandle, state: State<'_, AppState>) -> Result<Option<UpdateInfo>, String> {
    // Check if an update is already pending installation on restart
    let pending = state
        .db
        .get_setting("update_pending")
        .unwrap_or_default()
        .unwrap_or_default()
        == "true";

    if pending {
        return Ok(Some(UpdateInfo {
            version: "0.2.0".to_string(),
            notes: "Pending update ready to apply upon app restart".to_string(),
            download_size_bytes: 0,
        }));
    }

    Ok(None)
}
