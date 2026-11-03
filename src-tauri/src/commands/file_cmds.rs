// src-tauri/src/commands/file_cmds.rs
// Tauri IPC commands for file operations: pin/unpin, read, add, delete, and remote swarm download.

use tauri::{AppHandle, Emitter, State};
use iroh_blobs::store::Store;
use iroh_blobs::BlobFormat;

use crate::p2p::sync::{broadcast_catalog_event, SyncMessage};
use crate::storage::blob_tracker::{
    delete_blob, get_blob, list_all_blobs, set_pinned, touch_blob, upsert_blob, BlobRecord,
};
use crate::AppState;

/// List all tracked files and blobs from the SQLite database.
#[tauri::command]
pub async fn list_files(state: State<'_, AppState>) -> Result<Vec<BlobRecord>, String> {
    list_all_blobs(&state.db).map_err(|e| e.to_string())
}

/// Set a blob's pinned state (true = protect from eviction).
#[tauri::command]
pub async fn pin_file(hash: String, state: State<'_, AppState>) -> Result<(), String> {
    set_pinned(&state.db, &hash, true).map_err(|e| e.to_string())
}

/// Remove a blob's pinned protection.
#[tauri::command]
pub async fn unpin_file(hash: String, state: State<'_, AppState>) -> Result<(), String> {
    set_pinned(&state.db, &hash, false).map_err(|e| e.to_string())
}

/// Read a file's bytes from the local iroh-blobs store, automatically downloading
/// on-demand from connected peers/swarm if not currently cached on disk.
#[tauri::command]
pub async fn read_file_bytes(
    hash: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    const INLINE_LIMIT: u64 = 8 * 1024 * 1024; // 8 MB

    // Update access timestamp (LRU tracking).
    let _ = touch_blob(&state.db, &hash);

    let parsed_hash = hash
        .parse::<iroh_blobs::Hash>()
        .map_err(|e| format!("invalid hash '{hash}': {e}"))?;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    // 1. Try reading from local blob store first
    let local_res = engine
        .blobs
        .clone()
        .client()
        .read_to_bytes(parsed_hash)
        .await;

    let data = match local_res {
        Ok(d) => d,
        Err(_) => {
            // Blob is not locally stored yet: fetch on-demand from the swarm!
            let blob_rec = get_blob(&state.db, &hash).ok().flatten();
            let remotes: Vec<_> = engine.endpoint.remote_info_iter().collect();
            let mut download_ok = false;

            // Try the owner node first if known
            if let Some(rec) = &blob_rec {
                if let Some(owner) = &rec.owner_node_id {
                    if let Ok(owner_id) = owner.parse::<iroh::NodeId>() {
                        let addr = iroh::endpoint::NodeAddr::new(owner_id);
                        if let Ok(progress) = engine.blobs.clone().client().download(parsed_hash, addr).await {
                            let _ = progress.finish().await;
                            if engine.blobs.clone().client().read_to_bytes(parsed_hash).await.is_ok() {
                                download_ok = true;
                            }
                        }
                    }
                }
            }

            // Fallback: try all other connected swarm peers
            if !download_ok {
                for remote in remotes {
                    let addr = iroh::endpoint::NodeAddr::new(remote.node_id);
                    if let Ok(progress) = engine.blobs.clone().client().download(parsed_hash, addr).await {
                        let _ = progress.finish().await;
                        if engine.blobs.clone().client().read_to_bytes(parsed_hash).await.is_ok() {
                            download_ok = true;
                            break;
                        }
                    }
                }
            }

            if !download_ok {
                return Err(format!(
                    "Blob '{hash}' is not cached locally and no connected peers are currently hosting it."
                ));
            }

            engine
                .blobs
                .clone()
                .client()
                .read_to_bytes(parsed_hash)
                .await
                .map_err(|e| format!("blob read failed after swarm download: {e}"))?
        }
    };

    if data.len() as u64 <= INLINE_LIMIT {
        // Small enough to return inline.
        return Ok(data.to_vec());
    }

    // Large file: stream via events.
    const CHUNK_SIZE: usize = 256 * 1024; // 256 KB per event
    let total = data.len();
    let mut offset = 0usize;

    while offset < total {
        let end = (offset + CHUNK_SIZE).min(total);
        let chunk = &data[offset..end];

        app.emit(
            "file-chunk",
            serde_json::json!({
                "hash": hash,
                "offset": offset,
                "total": total,
                "data": chunk,
            }),
        )
        .map_err(|e| format!("event emit failed: {e}"))?;

        offset = end;
    }

    // Signal completion.
    app.emit(
        "file-chunk-done",
        serde_json::json!({ "hash": hash, "total": total }),
    )
    .map_err(|e| format!("event emit failed: {e}"))?;

    Ok(vec![])
}

/// Add a local file to the workspace blob store.
#[tauri::command]
pub async fn add_file(local_path: String, state: State<'_, AppState>) -> Result<String, String> {
    let path = std::path::Path::new(&local_path);

    let meta = std::fs::metadata(path).map_err(|e| format!("cannot stat '{}': {e}", local_path))?;
    if !meta.is_file() {
        return Err(format!("'{}' is not a regular file", local_path));
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    let data = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    let size_bytes = data.len() as u64;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;
    let owner_node_id = engine.node_id().to_string();

    let tag = engine
        .store
        .import_bytes(bytes::Bytes::from(data), BlobFormat::Raw)
        .await
        .map_err(|e| format!("blob import failed: {e}"))?;

    let hash_and_format = *tag.inner();
    let hash = hash_and_format.hash.to_hex().to_string();

    let _ = engine
        .store
        .set_tag(
            iroh_blobs::Tag::from(file_name.as_deref().unwrap_or(&hash)),
            Some(hash_and_format),
        )
        .await;

    // Record in SQLite with filename and owner node ID.
    upsert_blob(
        &state.db,
        &hash,
        file_name.as_deref(),
        size_bytes,
        false,
        Some(&owner_node_id),
    )
    .map_err(|e| e.to_string())?;

    // Broadcast file addition to the swarm
    let record = BlobRecord {
        hash: hash.clone(),
        name: file_name.clone(),
        size_bytes,
        last_accessed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        is_pinned: false,
        owner_node_id: Some(owner_node_id.clone()),
    };
    broadcast_catalog_event(&engine.endpoint, SyncMessage::FileAdded(record)).await;

    tracing::info!(hash = %hash, name = ?file_name, owner = %owner_node_id, bytes = size_bytes, "Added file to workspace");
    Ok(hash)
}

/// Upload a file to the shared cloud workspace via in-memory bytes (e.g. from UI file input).
#[tauri::command]
pub async fn upload_file_bytes(
    name: String,
    data: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let size_bytes = data.len() as u64;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;
    let owner_node_id = engine.node_id().to_string();

    let tag = engine
        .store
        .import_bytes(bytes::Bytes::from(data), BlobFormat::Raw)
        .await
        .map_err(|e| format!("blob import failed: {e}"))?;

    let hash_and_format = *tag.inner();
    let hash = hash_and_format.hash.to_hex().to_string();

    let _ = engine
        .store
        .set_tag(
            iroh_blobs::Tag::from(name.as_str()),
            Some(hash_and_format),
        )
        .await;

    upsert_blob(
        &state.db,
        &hash,
        Some(&name),
        size_bytes,
        false,
        Some(&owner_node_id),
    )
    .map_err(|e| e.to_string())?;

    // Broadcast file addition to the swarm
    let record = BlobRecord {
        hash: hash.clone(),
        name: Some(name.clone()),
        size_bytes,
        last_accessed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        is_pinned: false,
        owner_node_id: Some(owner_node_id.clone()),
    };
    broadcast_catalog_event(&engine.endpoint, SyncMessage::FileAdded(record)).await;

    tracing::info!(hash = %hash, name = %name, owner = %owner_node_id, bytes = size_bytes, "Uploaded file to workspace");
    Ok(hash)
}

/// Delete a file by hash.
/// The original uploader can delete their files.
/// If the owner is inactive or no longer exists in the swarm, members can force-clean up the orphaned file.
#[tauri::command]
pub async fn delete_file(
    hash: String,
    force_orphan: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let blob = get_blob(&state.db, &hash)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("file '{hash}' not found in workspace"))?;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;
    let my_node_id = engine.node_id().to_string();

    let is_owner = blob.owner_node_id.as_deref() == Some(&my_node_id);
    let force = force_orphan.unwrap_or(false);

    if !is_owner {
        let owner_str = blob.owner_node_id.as_deref().unwrap_or("unknown");
        let owner_is_online = engine
            .endpoint
            .remote_info_iter()
            .any(|r| r.node_id.to_string() == owner_str);

        if owner_is_online && !force {
            return Err(format!(
                "Permission denied: This file belongs to active user '{owner_str}'. Only the owner can delete active files."
            ));
        }

        tracing::info!(
            hash = %hash,
            owner = %owner_str,
            "Force-cleaning orphaned file (owner is inactive/departed)"
        );
    }

    // Delete record from SQLite
    delete_blob(&state.db, &hash).map_err(|e| e.to_string())?;

    // Delete blob data from local Iroh store
    if let Ok(parsed_hash) = hash.parse::<iroh_blobs::Hash>() {
        let _ = engine.blobs.clone().client().delete_blob(parsed_hash).await;
    }

    // Broadcast deletion event to the swarm
    broadcast_catalog_event(
        &engine.endpoint,
        SyncMessage::FileDeleted {
            hash: hash.clone(),
            deleter_node_id: my_node_id,
        },
    )
    .await;

    tracing::info!(hash = %hash, "Deleted file from workspace");
    Ok(())
}

/// Evict a blob from the local device's cache without deleting it from the cloud catalog.
#[tauri::command]
pub async fn evict_local_file(hash: String, state: State<'_, AppState>) -> Result<(), String> {
    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    if let Ok(parsed_hash) = hash.parse::<iroh_blobs::Hash>() {
        let _ = engine.blobs.clone().client().delete_blob(parsed_hash).await;
    }

    Ok(())
}

/// Save a blob to a specified local destination file path.
#[tauri::command]
pub async fn export_file(
    hash: String,
    dest_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let parsed_hash = hash
        .parse::<iroh_blobs::Hash>()
        .map_err(|e| format!("invalid hash '{hash}': {e}"))?;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    let data = engine
        .blobs
        .clone()
        .client()
        .read_to_bytes(parsed_hash)
        .await
        .map_err(|e| format!("blob read failed: {e}"))?;

    std::fs::write(&dest_path, data)
        .map_err(|e| format!("failed to write destination file '{dest_path}': {e}"))?;

    Ok(())
}
