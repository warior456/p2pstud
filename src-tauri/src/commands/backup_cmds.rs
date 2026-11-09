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
use crate::storage::db::Database;
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
    pub has_staged_installer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f64,
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

    // 1. Checkpoint WAL to flush pending writes
    {
        let conn = state.db.lock();
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(FULL);");
    }

    let dest_clone = dest.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = File::create(&dest_clone).map_err(|e| format!("Failed to create backup ZIP file: {e}"))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        if key_path.exists() {
            let key_data = std::fs::read(&key_path)
                .map_err(|e| format!("Failed to read iroh_secret.key: {e}"))?;
            zip.start_file("iroh_secret.key", options)
                .map_err(|e| format!("Failed to write key to ZIP: {e}"))?;
            zip.write_all(&key_data)
                .map_err(|e| format!("Failed to write key content to ZIP: {e}"))?;
        }

        let db_data = std::fs::read(&db_path)
            .map_err(|e| format!("Failed to read p2pstud.db: {e}"))?;
        zip.start_file("p2pstud.db", options)
            .map_err(|e| format!("Failed to write DB to ZIP: {e}"))?;
        zip.write_all(&db_data)
            .map_err(|e| format!("Failed to write DB content to ZIP: {e}"))?;

        zip.finish()
            .map_err(|e| format!("Failed to finalize backup ZIP: {e}"))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Backup worker failed: {e}"))??;

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

    if let Some(db_data) = db_bytes {
        let temp_staging_path = data_dir.join("p2pstud_restore_staging.tmp");
        std::fs::write(&temp_staging_path, &db_data)
            .map_err(|e| format!("Failed to stage restored p2pstud.db: {e}"))?;

        // Verify SQLite integrity of restored database before applying
        {
            let staging_conn = rusqlite::Connection::open(&temp_staging_path)
                .map_err(|e| format!("Invalid restored SQLite database: {e}"))?;
            let integrity: String = staging_conn
                .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
                .map_err(|e| format!("Failed running integrity check on restored database: {e}"))?;
            if integrity != "ok" {
                let _ = std::fs::remove_file(&temp_staging_path);
                return Err(format!("Restored database integrity check failed: {integrity}"));
            }

            // Safely apply using SQLite online backup API into active connection
            let mut target_conn = state.db.lock();
            let backup = rusqlite::backup::Backup::new(&staging_conn, &mut target_conn)
                .map_err(|e| format!("Failed initializing SQLite backup restore: {e}"))?;
            backup.run_to_completion(10, std::time::Duration::from_millis(10), None)
                .map_err(|e| format!("Failed copying restored database pages: {e}"))?;
        }
        let _ = std::fs::remove_file(&temp_staging_path);
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

/// Check for application updates against https://github.com/warior456/p2pstud/releases.
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// Parse a version string, tolerating a leading `v` and surrounding whitespace.
fn parse_version(raw: &str) -> Option<semver::Version> {
    semver::Version::parse(raw.trim().trim_start_matches('v')).ok()
}

/// True when `candidate` is strictly newer than `current`.
fn is_version_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => candidate.trim() != current.trim(),
    }
}

/// Forget any staged installer reference so stale "installer ready" banners disappear.
fn clear_staged_update_settings(db: &Database) {
    let _ = db.set_setting("update_pending", "false");
    let _ = db.delete_setting("update_installer_path");
    let _ = db.delete_setting("update_installer_hash");
}

/// Reject update URLs that are not HTTPS GitHub-hosted release assets.
fn validate_update_download_url(raw: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(raw.trim()).map_err(|e| format!("Invalid update URL: {e}"))?;
    if parsed.scheme() != "https" {
        return Err("Update URL must use HTTPS".to_string());
    }
    match parsed.host_str() {
        Some("github.com") => Ok(parsed),
        Some(host) if host.ends_with(".githubusercontent.com") => Ok(parsed),
        Some(host) => Err(format!("Refusing to download update from untrusted host: {host}")),
        None => Err("Update URL has no host".to_string()),
    }
}

/// Reduce a GitHub release asset name to a safe bare file name.
fn sanitize_update_asset_name(raw: &str) -> Option<String> {
    let name = Path::new(raw.trim()).file_name()?.to_string_lossy().to_string();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    let invalid = name.chars().any(|c| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    });
    if invalid {
        None
    } else {
        Some(name)
    }
}

/// Stream a file through BLAKE3 and return its hex digest (matches `iroh_blobs::Hash`).
fn hash_file_streaming(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed reading '{}': {e}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed reading '{}': {e}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Remove stale staged installers and their settings.
///
/// Keeps exactly one artifact only when an update for a newer version is pending and the
/// staged file still exists inside the app's `updates` directory.
pub fn cleanup_stale_update_artifacts(db: &Database, data_dir: &Path) {
    let updates_dir = data_dir.join("updates");
    let current_version = env!("CARGO_PKG_VERSION");

    let staged_path = db
        .get_setting("update_installer_path")
        .ok()
        .flatten()
        .unwrap_or_default();
    let staged_file = PathBuf::from(staged_path.trim_start_matches(r"\\?\"));
    let pending = db
        .get_setting("update_pending")
        .ok()
        .flatten()
        .as_deref()
        == Some("true");
    let target_version = db
        .get_setting("update_target_version")
        .ok()
        .flatten()
        .unwrap_or_default();

    let staged_inside_updates = !staged_path.trim().is_empty() && staged_file.starts_with(&updates_dir);
    let keep_staged = pending
        && staged_inside_updates
        && staged_file.is_file()
        && is_version_newer(&target_version, current_version);

    if updates_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&updates_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let is_kept = keep_staged
                    && path.file_name().is_some()
                    && path.file_name() == staged_file.file_name();
                if !is_kept {
                    if let Err(e) = std::fs::remove_file(&path) {
                        tracing::warn!(path = %path.display(), error = %e, "Failed to remove stale update artifact");
                    } else {
                        tracing::info!(path = %path.display(), "Removed stale update artifact");
                    }
                }
            }
        }
    }

    if !keep_staged {
        clear_staged_update_settings(db);
    }
}

/// Internal helper to check GitHub releases and stage update settings into SQLite.
pub async fn check_and_stage_app_update(db: &Database, force: bool) -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let staged_path = db
        .get_setting("update_installer_path")
        .unwrap_or_default()
        .unwrap_or_default();
    let staged_exists = !staged_path.trim().is_empty()
        && Path::new(staged_path.trim_start_matches(r"\\?\")).is_file();

    let pending_raw = db
        .get_setting("update_pending")
        .unwrap_or_default()
        .unwrap_or_default()
        == "true";

    let pending_version = db
        .get_setting("update_target_version")
        .unwrap_or_default()
        .unwrap_or_else(|| current_version.clone());

    // A staged installer is only meaningful while its target release is still newer
    // than the running build. Otherwise the update was already installed (or is stale).
    let pending = pending_raw && staged_exists && is_version_newer(&pending_version, &current_version);
    if pending_raw && !pending {
        tracing::info!("Discarding stale pending-update state");
        clear_staged_update_settings(db);
    }
    let has_staged_installer = pending && staged_exists;

    if pending {
        return Ok(UpdateInfo {
            current_version: current_version.clone(),
            version: pending_version,
            notes: "Pending update ready to apply upon app restart".to_string(),
            download_size_bytes: db
                .get_setting("update_asset_size")
                .unwrap_or_default()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0),
            is_pending: true,
            update_available: true,
            has_staged_installer,
        });
    }

    // Check if user dismissed updates for this session (ignored if manual check was forced)
    let dismissed = if force {
        false
    } else {
        db.get_setting("update_dismissed_session")
            .unwrap_or_default()
            .unwrap_or_default()
            == "true"
    };

    // Query GitHub releases for https://github.com/warior456/p2pstud/releases
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err(e.to_string()),
    };

    let resp = match client
        .get("https://api.github.com/repos/warior456/p2pstud/releases/latest")
        .header("User-Agent", format!("p2pstud/{current_version}"))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to connect to GitHub releases API (possibly offline)");
            return Ok(UpdateInfo {
                current_version: current_version.clone(),
                version: current_version,
                notes: "Could not reach update server (offline or rate-limited)".to_string(),
                download_size_bytes: 0,
                is_pending: false,
                update_available: false,
                has_staged_installer,
            });
        }
    };

    let (remote_ver_str, notes_str, matched_asset) = if resp.status().is_success() {
        if let Ok(release) = resp.json::<GitHubRelease>().await {
            let remote_ver = release.tag_name.trim().trim_start_matches('v').to_string();
            let matched = {
                #[cfg(target_os = "windows")]
                {
                    release
                        .assets
                        .iter()
                        .find(|a| a.name.to_lowercase().ends_with("-setup.exe"))
                        .or_else(|| {
                            release
                                .assets
                                .iter()
                                .find(|a| a.name.to_lowercase().ends_with(".msi"))
                        })
                        .or_else(|| {
                            release
                                .assets
                                .iter()
                                .find(|a| {
                                    let name = a.name.to_lowercase();
                                    (name == "p2pstud-windows-x86_64.exe" || name == "p2pstud.exe")
                                        && !name.starts_with("anchor")
                                })
                        })
                        .or_else(|| {
                            release.assets.iter().find(|a| {
                                let name = a.name.to_lowercase();
                                name.ends_with(".exe")
                                    && !name.starts_with("anchor")
                            })
                        })
                        .cloned()
                }
                #[cfg(target_os = "macos")]
                {
                    release
                        .assets
                        .iter()
                        .find(|a| {
                            let name = a.name.to_lowercase();
                            name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".dmg")
                        })
                        .cloned()
                }
                #[cfg(target_os = "linux")]
                {
                    release
                        .assets
                        .iter()
                        .find(|a| {
                            let name = a.name.to_lowercase();
                            (name == "p2pstud-linux-x86_64" || name == "p2pstud") && !name.starts_with("anchor")
                        })
                        .or_else(|| {
                            release.assets.iter().find(|a| {
                                let name = a.name.to_lowercase();
                                (name.ends_with(".appimage") || name.ends_with(".deb")) && !name.starts_with("anchor")
                            })
                        })
                        .cloned()
                }
                #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
                {
                    None
                }
            };
            (remote_ver, release.body.unwrap_or_else(|| "New release available on GitHub.".to_string()), matched)
        } else {
            (current_version.clone(), String::new(), None)
        }
    } else {
        tracing::warn!(status = %resp.status(), "GitHub releases API returned non-success status, attempting web redirect fallback...");
        // Fallback when GitHub API is rate-limited (HTTP 403) or blocked: follow HTML redirect
        if let Ok(redir_resp) = client.get("https://github.com/warior456/p2pstud/releases/latest").send().await {
            let final_url = redir_resp.url().as_str();
            if let Some(tag_part) = final_url.split("/releases/tag/").nth(1) {
                let tag = tag_part.trim().trim_matches('/');
                let ver = tag.trim_start_matches('v').to_string();

                #[cfg(target_os = "windows")]
                let (asset_name, dl_url) = (
                    format!("p2pstud_{ver}_x64-setup.exe"),
                    format!("https://github.com/warior456/p2pstud/releases/download/{tag}/p2pstud_{ver}_x64-setup.exe"),
                );
                #[cfg(target_os = "macos")]
                let (asset_name, dl_url) = (
                    format!("p2pstud_{ver}_aarch64.dmg"),
                    format!("https://github.com/warior456/p2pstud/releases/download/{tag}/p2pstud_{ver}_aarch64.dmg"),
                );
                #[cfg(target_os = "linux")]
                let (asset_name, dl_url) = (
                    format!("p2pstud_{ver}_amd64.AppImage"),
                    format!("https://github.com/warior456/p2pstud/releases/download/{tag}/p2pstud_{ver}_amd64.AppImage"),
                );

                let asset = Some(GitHubAsset {
                    name: asset_name,
                    size: 15 * 1024 * 1024,
                    browser_download_url: dl_url,
                });

                (ver, format!("New release {tag} available on GitHub."), asset)
            } else {
                (current_version.clone(), format!("Update server returned status {}", resp.status()), None)
            }
        } else {
            (current_version.clone(), format!("Update server returned status {}", resp.status()), None)
        }
    };

    let is_newer = is_version_newer(&remote_ver_str, &current_version);

    // Only persist download metadata for an update we would actually install. Storing it
    // for an older/equal release risks downloading the wrong installer later.
    if is_newer {
        if let Some(asset) = &matched_asset {
            match (validate_update_download_url(&asset.browser_download_url), sanitize_update_asset_name(&asset.name)) {
                (Ok(_), Some(safe_name)) => {
                    let _ = db.set_setting("update_target_version", &remote_ver_str);
                    let _ = db.set_setting("update_download_url", &asset.browser_download_url);
                    let _ = db.set_setting("update_asset_name", &safe_name);
                    let _ = db.set_setting("update_asset_size", &asset.size.to_string());
                }
                (Err(e), _) => tracing::warn!(error = %e, "Ignoring update asset with unsafe download URL"),
                (_, None) => tracing::warn!(asset = %asset.name, "Ignoring update asset with unsafe file name"),
            }
        }
    }

    if is_newer && !dismissed {
        if let Some(asset) = matched_asset {
            return Ok(UpdateInfo {
                current_version,
                version: remote_ver_str,
                notes: notes_str,
                download_size_bytes: asset.size,
                is_pending: false,
                update_available: true,
                has_staged_installer,
            });
        }
    }
    let dl_size = matched_asset.as_ref().map(|a| a.size).unwrap_or(0);
    Ok(UpdateInfo {
        current_version: current_version.clone(),
        version: remote_ver_str,
        notes: if is_newer && matched_asset.is_none() {
            "New version found, but no compatible installer artifact was found for this platform.".to_string()
        } else {
            notes_str
        },
        download_size_bytes: dl_size,
        is_pending: false,
        update_available: false,
        has_staged_installer,
    })
}

#[tauri::command]
pub async fn check_app_update(force: Option<bool>, _app: AppHandle, state: State<'_, AppState>) -> Result<UpdateInfo, String> {
    check_and_stage_app_update(&state.db, force.unwrap_or(false)).await
}

/// Download application update installer from GitHub releases to disk with real-time progress.
#[tauri::command]
pub async fn download_app_update(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let download_url = state
        .db
        .get_setting("update_download_url")
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update download URL found. Please check for updates first.".to_string())?;

    // Validate the stored URL before ever touching the network.
    let validated_url = validate_update_download_url(&download_url)?;

    let raw_asset_name = state
        .db
        .get_setting("update_asset_name")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "p2pstud-update-installer.exe".to_string());

    let asset_name = sanitize_update_asset_name(&raw_asset_name)
        .ok_or_else(|| "Stored update asset name is unsafe; please check for updates again.".to_string())?;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

    let updates_dir = data_dir.join("updates");
    tokio::fs::create_dir_all(&updates_dir)
        .await
        .map_err(|e| format!("Failed to create updates directory: {e}"))?;

    let dest_path = updates_dir.join(&asset_name);

    // Drop installers left over from previous releases before writing the new one.
    if let Ok(mut entries) = tokio::fs::read_dir(&updates_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() && path != dest_path {
                let _ = tokio::fs::remove_file(&path).await;
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    tracing::info!(url = %validated_url, dest = %dest_path.display(), "Downloading application update installer");

    let mut resp = client
        .get(validated_url)
        .header("User-Agent", "p2pstud")
        .send()
        .await
        .map_err(|e| format!("Failed to download update installer: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with HTTP status: {}", resp.status()));
    }

    let total_size = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    use tokio::io::AsyncWriteExt;
    let mut dest_file = tokio::fs::File::create(&dest_path)
        .await
        .map_err(|e| format!("Failed to create update file: {e}"))?;

    let mut last_progress_pct = 0.0;

    let _ = app.emit(
        "update-download-progress",
        UpdateDownloadProgress {
            downloaded_bytes: 0,
            total_bytes: total_size,
            percentage: 0.0,
        },
    );

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("Error downloading update chunk: {e}"))?
    {
        dest_file
            .write_all(&chunk)
            .await
            .map_err(|e| format!("Failed writing chunk to disk: {e}"))?;
        downloaded += chunk.len() as u64;

        let pct = if total_size > 0 {
            (downloaded as f64 / total_size as f64) * 100.0
        } else {
            0.0
        };

        if pct - last_progress_pct >= 1.0 || downloaded == total_size {
            last_progress_pct = pct;
            let _ = app.emit(
                "update-download-progress",
                UpdateDownloadProgress {
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    percentage: pct,
                },
            );
        }
    }

    dest_file
        .flush()
        .await
        .map_err(|e| format!("Failed to flush update file: {e}"))?;
    drop(dest_file);

    // Hash on a blocking thread while streaming, so a ~15 MB installer is never fully
    // buffered in memory just to verify it.
    let hash_path = dest_path.clone();
    let hash = tokio::task::spawn_blocking(move || hash_file_streaming(&hash_path))
        .await
        .map_err(|e| format!("Installer verification worker failed: {e}"))??;

    let _ = state.db.set_setting("update_installer_path", &dest_path.to_string_lossy());
    let _ = state.db.set_setting("update_installer_hash", &hash);
    let _ = state.db.set_setting("update_pending", "true");
    let _ = app.emit("update-downloaded", ());

    tracing::info!(dest = %dest_path.display(), hash = %hash, "Application update successfully downloaded and staged");
    Ok("Update downloaded and verified. Ready to apply.".to_string())
}

/// Apply the pending update by spawning the installer and exiting the app.
#[tauri::command]
pub async fn install_app_update(_app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let installer_path_str = state
        .db
        .get_setting("update_installer_path")
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No downloaded update installer found.".to_string())?;

    let data_dir = _app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    let updates_dir = data_dir.join("updates");

    let installer_path = PathBuf::from(&installer_path_str);
    let canonical_installer = installer_path.canonicalize().map_err(|_| {
        clear_staged_update_settings(&state.db);
        format!("Installer file not found at: {}", installer_path.display())
    })?;

    let canonical_updates_dir = updates_dir.canonicalize().map_err(|e| {
        format!("Updates directory cannot be resolved: {e}")
    })?;

    // Security check 1: Path must be strictly within updates_dir
    if !canonical_installer.starts_with(&canonical_updates_dir) {
        clear_staged_update_settings(&state.db);
        return Err("Security violation: installer is located outside the updates directory".to_string());
    }

    // Security check 2: Verify binary hash matches update_installer_hash recorded during download
    let expected_hash = match state.db.get_setting("update_installer_hash").map_err(|e| e.to_string())? {
        Some(hash) if !hash.trim().is_empty() => hash,
        _ => {
            clear_staged_update_settings(&state.db);
            return Err("Missing verified hash for pending update; please check for updates again.".to_string());
        }
    };

    let hash_path = canonical_installer.clone();
    let actual_hash = tokio::task::spawn_blocking(move || hash_file_streaming(&hash_path))
        .await
        .map_err(|e| format!("Installer verification worker failed: {e}"))??;
    if actual_hash != expected_hash {
        clear_staged_update_settings(&state.db);
        let _ = std::fs::remove_file(&canonical_installer);
        return Err("Security verification failed: staged installer binary checksum does not match expected release digest!".to_string());
    }

    tracing::info!(path = %canonical_installer.display(), "Launching installer and restarting application");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x00000010;

        let pid = std::process::id();
        let current_exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("p2pstud.exe"))
            .to_string_lossy()
            .replace('\'', "''");

        let clean_installer_str = canonical_installer
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\'', "''");

        let script = if installer_path_str.ends_with(".msi") {
            format!(
                "$Host.UI.RawUI.WindowTitle = 'P2P Campus Workspace Updater'; \
                 Clear-Host; \
                 Write-Host '===================================================' -ForegroundColor Cyan; \
                 Write-Host '   P2P Campus Workspace Updater' -ForegroundColor Cyan; \
                 Write-Host '===================================================' -ForegroundColor Cyan; \
                 Write-Host ''; \
                 Write-Host 'Closing running P2PStud process...' -ForegroundColor Yellow; \
                 try {{ Stop-Process -Id {pid} -Force -ErrorAction SilentlyContinue }} catch {{}}; \
                 Start-Sleep -Milliseconds 600; \
                 Get-Process -Name 'p2pstud' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue; \
                 Start-Sleep -Milliseconds 600; \
                 Write-Host 'Applying MSI update silently...' -ForegroundColor Green; \
                 $proc = Start-Process -FilePath 'msiexec.exe' -ArgumentList '/i', '\"{}\"', '/quiet', '/norestart' -Wait -PassThru; \
                 Write-Host \"Installer finished with exit code $($proc.ExitCode).\" -ForegroundColor Cyan; \
                 Remove-Item -LiteralPath '{clean_installer_str}' -Force -ErrorAction SilentlyContinue; \
                 Start-Sleep -Milliseconds 800; \
                 $f1 = [System.Environment]::GetFolderPath('LocalApplicationData') + '\\p2pstud\\p2pstud.exe'; \
                 $f2 = [System.Environment]::GetFolderPath('LocalApplicationData') + '\\Programs\\p2pstud\\p2pstud.exe'; \
                 $target = '{current_exe}'; \
                 if (Test-Path $f1) {{ $target = $f1 }} elseif (Test-Path $f2) {{ $target = $f2 }}; \
                 if (Test-Path $target) {{ Write-Host \"Relaunching $target...\" -ForegroundColor Green; Start-Process -FilePath $target }}",
                clean_installer_str
            )
        } else if installer_path_str.to_lowercase().ends_with("-setup.exe") {
            format!(
                "$Host.UI.RawUI.WindowTitle = 'P2P Campus Workspace Updater'; \
                 Clear-Host; \
                 Write-Host '===================================================' -ForegroundColor Cyan; \
                 Write-Host '   P2P Campus Workspace Updater' -ForegroundColor Cyan; \
                 Write-Host '===================================================' -ForegroundColor Cyan; \
                 Write-Host ''; \
                 Write-Host 'Closing running P2PStud process...' -ForegroundColor Yellow; \
                 try {{ Stop-Process -Id {pid} -Force -ErrorAction SilentlyContinue }} catch {{}}; \
                 Start-Sleep -Milliseconds 600; \
                 Get-Process -Name 'p2pstud' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue; \
                 Start-Sleep -Milliseconds 600; \
                 Write-Host 'Applying update silently...' -ForegroundColor Green; \
                 $proc = Start-Process -FilePath '{clean_installer_str}' -ArgumentList '/S' -Wait -PassThru; \
                 Write-Host \"Setup completed (Exit code: $($proc.ExitCode)).\" -ForegroundColor Cyan; \
                 Remove-Item -LiteralPath '{clean_installer_str}' -Force -ErrorAction SilentlyContinue; \
                 Start-Sleep -Milliseconds 800; \
                 $f1 = [System.Environment]::GetFolderPath('LocalApplicationData') + '\\p2pstud\\p2pstud.exe'; \
                 $f2 = [System.Environment]::GetFolderPath('LocalApplicationData') + '\\Programs\\p2pstud\\p2pstud.exe'; \
                 $target = '{current_exe}'; \
                 if (Test-Path $f1) {{ $target = $f1 }} elseif (Test-Path $f2) {{ $target = $f2 }}; \
                 if (Test-Path $target) {{ \
                     Write-Host \"Relaunching updated application ($target)...\" -ForegroundColor Green; \
                     Start-Process -FilePath $target; \
                 }} else {{ \
                     Start-Process -FilePath '{clean_installer_str}' -ArgumentList '/S'; \
                 }}",
            )
        } else {
            // Direct standalone binary replacement (.exe)
            format!(
                "$Host.UI.RawUI.WindowTitle = 'P2P Campus Workspace Updater'; \
                 Clear-Host; \
                 Write-Host '===================================================' -ForegroundColor Cyan; \
                 Write-Host '   P2P Campus Workspace Updater' -ForegroundColor Cyan; \
                 Write-Host '===================================================' -ForegroundColor Cyan; \
                 Write-Host ''; \
                 Write-Host 'Closing running P2PStud process...' -ForegroundColor Yellow; \
                 try {{ Stop-Process -Id {pid} -Force -ErrorAction SilentlyContinue }} catch {{}}; \
                 Start-Sleep -Milliseconds 600; \
                 Get-Process -Name 'p2pstud' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue; \
                 Start-Sleep -Milliseconds 600; \
                 $target = '{current_exe}'; \
                 $f1 = [System.Environment]::GetFolderPath('LocalApplicationData') + '\\p2pstud\\p2pstud.exe'; \
                 $f2 = [System.Environment]::GetFolderPath('LocalApplicationData') + '\\Programs\\p2pstud\\p2pstud.exe'; \
                 if (Test-Path $f1) {{ $target = $f1 }} elseif (Test-Path $f2) {{ $target = $f2 }}; \
                 Write-Host \"Replacing application binary ($target)...\" -ForegroundColor Green; \
                 $replaced = $false; \
                 for ($i = 0; $i -lt 20; $i++) {{ \
                     try {{ \
                         Copy-Item -Path '{clean_installer_str}' -Destination $target -Force; \
                         $replaced = $true; \
                         break; \
                     }} catch {{ \
                         Start-Sleep -Milliseconds 300; \
                     }} \
                 }}; \
                 if ($replaced) {{ \
                     Write-Host 'Binary successfully updated! Relaunching application...' -ForegroundColor Green; \
                     Remove-Item -LiteralPath '{clean_installer_str}' -Force -ErrorAction SilentlyContinue; \
                     Start-Process -FilePath $target; \
                 }} else {{ \
                     Write-Host 'Failed to overwrite binary. Relaunching existing executable...' -ForegroundColor Red; \
                     Start-Sleep -Seconds 2; \
                     Start-Process -FilePath $target; \
                 }}",
            )
        };

        let spawn_res = std::process::Command::new("powershell.exe")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&script)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn();

        match spawn_res {
            Ok(_) => {
                clear_staged_update_settings(&state.db);
                tracing::info!("Updater console spawned successfully, exiting host process");
                std::process::exit(0);
            }
            Err(e) => {
                // Keep the staged installer usable so the user can retry the install.
                let _ = state.db.set_setting("update_pending", "true");
                tracing::error!(error = %e, "Failed to spawn updater process");
                Err(format!("Failed to launch updater: {e}"))
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if installer_path_str.ends_with(".tar.gz") || installer_path_str.ends_with(".tgz") {
            let _ = std::process::Command::new("tar")
                .args(["-xzf", &installer_path_str, "-C", "/Applications"])
                .status();
            let _ = std::process::Command::new("open")
                .arg("/Applications/p2pstud.app")
                .spawn();
        } else if installer_path_str.ends_with(".dmg") {
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!(
                    "hdiutil attach -nobrowse '{0}' -mountpoint /tmp/p2pstud_mount && cp -R /tmp/p2pstud_mount/p2pstud.app /Applications/ && hdiutil detach /tmp/p2pstud_mount && open /Applications/p2pstud.app",
                    installer_path_str
                ))
                .spawn();
        } else {
            let _ = std::process::Command::new("open")
                .arg(&installer_path)
                .spawn();
        }
        clear_staged_update_settings(&state.db);
        std::process::exit(0);
    }

    #[cfg(target_os = "linux")]
    {
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("p2pstud"));
        if installer_path_str.ends_with(".appimage") {
            let _ = std::process::Command::new("chmod")
                .arg("+x")
                .arg(&installer_path)
                .status();
            let _ = std::process::Command::new(&installer_path).spawn();
        } else if installer_path_str.ends_with(".deb") {
            let _ = std::process::Command::new("pkexec")
                .args(["dpkg", "-i", &installer_path_str])
                .spawn();
        } else {
            // Direct standalone binary replacement
            let _ = std::process::Command::new("chmod")
                .arg("+x")
                .arg(&installer_path)
                .status();
            let _ = std::fs::copy(&installer_path, &current_exe);
            let _ = std::process::Command::new(&current_exe).spawn();
        }
        clear_staged_update_settings(&state.db);
        std::process::exit(0);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        app.restart();
        Ok(())
    }
}

/// Dismiss the update notification for the current session.
#[tauri::command]
pub async fn dismiss_app_update(state: State<'_, AppState>) -> Result<(), String> {
    let _ = state.db.set_setting("update_dismissed_session", "true");
    Ok(())
}

/// Clear session dismissal flag on application startup so user is notified of updates in new sessions.
pub fn reset_update_dismissal_on_startup(db: &Database) {
    let _ = db.delete_setting("update_dismissed_session");
}

/// Gracefully exit or restart the application, optionally applying a pending update.
#[tauri::command]
pub async fn confirm_app_exit(
    install_update: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if install_update {
        install_app_update(app, state).await?;
    } else {
        std::process::exit(0);
    }
    Ok(())
}
