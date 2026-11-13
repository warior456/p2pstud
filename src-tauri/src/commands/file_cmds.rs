// src-tauri/src/commands/file_cmds.rs
// Tauri IPC commands for file operations: pin/unpin, read, add, delete, vote delete, and remote swarm download.

use tauri::{AppHandle, Emitter, Manager, State};

use crate::p2p::engine::IrohEngine;
use crate::p2p::sync::{broadcast_catalog_event, SyncMessage};
use crate::storage::blob_tracker::{
    cast_delete_vote, cast_keep_vote, delete_blob, get_blob, get_seed_count,
    list_all_blobs_filtered, record_peer_seed, remove_delete_vote, remove_keep_vote, rename_blob,
    set_pinned, touch_blob, upsert_blob, BlobRecord, DeletionTombstone,
};
use crate::storage::hosting_worker::is_blob_cached;
use crate::AppState;

/// Resolve the blob store for upload-only operations.
///
/// Uploads must work while the P2P engine is still starting (or fully offline),
/// so fall back to the preloaded store and the persisted identity. The returned
/// engine is `None` when there is no running node to broadcast catalog events.
async fn resolve_upload_context(
    state: &AppState,
) -> Result<(iroh_blobs::store::fs::FsStore, String, Option<IrohEngine>), String> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        return Ok((
            engine.store.clone(),
            engine.node_id().to_string(),
            Some(engine.clone()),
        ));
    }
    drop(engine_guard);

    let store_guard = state.store.read().await;
    let store = store_guard
        .clone()
        .ok_or_else(|| "Local storage is not ready yet. Please retry in a moment.".to_string())?;
    drop(store_guard);

    let key = crate::p2p::identity::load_or_create_secret_key(&state.data_dir)
        .map_err(|e| format!("failed to load local identity: {e}"))?;
    Ok((store, key.public().to_string(), None))
}

/// Broadcast a catalog event only when a running engine is available.
async fn broadcast_catalog_event_opt(engine: &Option<IrohEngine>, msg: SyncMessage) {
    if let Some(engine) = engine.as_ref() {
        broadcast_catalog_event(&engine.endpoint, &engine.sync_alpn, msg).await;
    }
}

/// Resolve this node's identity, the running engine (if any), and the persisted
/// secret key. Destructive/local actions must keep working while the engine is
/// starting or the app is offline.
async fn resolve_identity_context(
    state: &AppState,
) -> Result<(String, Option<IrohEngine>, iroh::SecretKey), String> {
    let engine_guard = state.engine.read().await;
    if let Some(engine) = engine_guard.as_ref() {
        return Ok((
            engine.node_id().to_string(),
            Some(engine.clone()),
            engine.secret_key.clone(),
        ));
    }
    drop(engine_guard);

    let key = crate::p2p::identity::load_or_create_secret_key(&state.data_dir)
        .map_err(|e| format!("failed to load local identity: {e}"))?;
    let node_id = key.public().to_string();
    Ok((node_id, None, key))
}

/// Resolve the local blob store without requiring the P2P engine.
async fn resolve_local_store(state: &AppState) -> Option<iroh_blobs::store::fs::FsStore> {
    {
        let engine_guard = state.engine.read().await;
        if let Some(engine) = engine_guard.as_ref() {
            return Some(engine.store.clone());
        }
    }
    state.store.read().await.clone()
}

/// Progress event emitted during local file additions and ZIP archive extractions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportProgressPayload {
    pub import_id: String,
    pub archive_name: String,
    pub current_entry: usize,
    pub total_entries: usize,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub current_file: String,
    pub bytes_per_sec: u64,
    pub status: String,
    pub error: Option<String>,
}

/// Helper function to reliably emit import progress to both the global app handle
/// and explicitly to the main webview window.
pub(crate) fn emit_import_progress(app: Option<&AppHandle>, payload: &ImportProgressPayload) {
    if let Some(app) = app {
        let _ = app.emit("import-progress", payload);
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.emit("import-progress", payload);
        }
    }
}

/// List all tracked files and blobs from the SQLite database with live cache status.
/// Works 100% offline using the preloaded local store and SQLite metadata.
#[tauri::command]
pub async fn list_files(state: State<'_, AppState>) -> Result<Vec<BlobRecord>, String> {
    let mut blobs = list_all_blobs_filtered(&state.db, true).map_err(|e| e.to_string())?;

    // Determine our node ID (from running engine or persistent secret key on disk)
    let my_node_id = {
        let guard = state.engine.read().await;
        if let Some(engine) = guard.as_ref() {
            Some(engine.node_id().to_string())
        } else {
            crate::p2p::identity::load_or_create_secret_key(&state.data_dir)
                .ok()
                .map(|k| k.public().to_string())
        }
    };

    let engine_guard = state.engine.read().await;
    let store_guard = state.store.read().await;

    let maybe_store: Option<&iroh_blobs::store::fs::FsStore> = if let Some(engine) = engine_guard.as_ref() {
        Some(&engine.store)
    } else if let Some(store) = store_guard.as_ref() {
        Some(store)
    } else {
        None
    };

    let blob_dir = match state.db.get_setting("custom_blob_dir") {
        Ok(Some(custom)) if !custom.trim().is_empty() => std::path::PathBuf::from(custom.trim()),
        _ => state.data_dir.join("blobs"),
    };
    let data_dir = blob_dir.join("data");

    for blob in &mut blobs {
        if let Ok(parsed_hash) = blob.hash.parse::<iroh_blobs::Hash>() {
            let mut is_cached = false;
            if let Some(store) = maybe_store {
                is_cached = is_blob_cached(store, &parsed_hash).await;
            }
            if !is_cached {
                let disk_file = data_dir.join(format!("{}.data", blob.hash));
                let fallback_disk_file = blob_dir.join(format!("{}.data", blob.hash));
                let found_path = if disk_file.exists() {
                    Some(disk_file)
                } else if fallback_disk_file.exists() {
                    Some(fallback_disk_file)
                } else {
                    None
                };

                if let Some(f) = found_path {
                    is_cached = true;
                    // Auto-heal / re-index into store asynchronously
                    if let Some(store) = maybe_store {
                        let s = store.clone();
                        tokio::spawn(async move {
                            if let Ok(bytes) = tokio::fs::read(&f).await {
                                let _ = s.blobs().add_bytes(bytes).await;
                            }
                        });
                    }
                }
            }
            blob.is_cached = is_cached;
            // Keep the presence marker consistent with what is actually on disk.
            if is_cached {
                if !blob.written_at.map(|w| w > 0).unwrap_or(false)
                    && crate::storage::blob_tracker::mark_blob_local(&state.db, &blob.hash).is_ok()
                {
                    blob.written_at = Some(crate::storage::blob_tracker::now_unix());
                }
            } else if blob.written_at.map(|w| w > 0).unwrap_or(false) {
                // Ghost record: SQLite has written_at > 0, but the physical bytes are missing from store and disk.
                // Reconcile SQLite so it accurately reflects that this node does NOT have the local content cached.
                let _ = crate::storage::blob_tracker::mark_blob_remote(&state.db, &blob.hash);
                blob.written_at = Some(0);
            }
            blob.seed_count = get_seed_count(
                &state.db,
                &blob.hash,
                my_node_id.as_deref(),
                blob.is_cached,
            )
            .unwrap_or(if blob.is_cached { 1 } else { 0 });
        }
    }
    Ok(blobs)
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

/// Block a file locally, immediately purging physical bytes from disk and preventing
/// future catalog auto-sync or background hosting of this content.
#[tauri::command]
pub async fn block_file(
    hash: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::storage::blob_tracker::block_blob(&state.db, &hash, reason.as_deref())
        .map_err(|e| format!("failed to block file: {e}"))?;

    if let Some(store) = resolve_local_store(&state).await {
        let _ = store.tags().delete(hash.as_bytes()).await;
    }

    Ok(())
}

/// Unblock a previously blocked file hash.
#[tauri::command]
pub async fn unblock_file(hash: String, state: State<'_, AppState>) -> Result<(), String> {
    crate::storage::blob_tracker::unblock_blob(&state.db, &hash)
        .map_err(|e| format!("failed to unblock file: {e}"))
}

/// List all locally blocked file hashes.
#[tauri::command]
pub async fn list_blocked_files(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    crate::storage::blob_tracker::list_blocked_blobs(&state.db)
        .map_err(|e| format!("failed to list blocked files: {e}"))
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

    if crate::storage::blob_tracker::is_quarantined(&state.db, &hash).unwrap_or(false) {
        return Err("File is quarantined by community security consensus. Reading bytes is restricted.".to_string());
    }
    if crate::storage::blob_tracker::is_blob_blocked(&state.db, &hash).unwrap_or(false) {
        return Err("File is blocked by local security policy.".to_string());
    }

    // Update access timestamp (LRU tracking).
    let _ = touch_blob(&state.db, &hash);

    let parsed_hash = hash
        .parse::<iroh_blobs::Hash>()
        .map_err(|e| format!("invalid hash '{hash}': {e}"))?;

    // 1. Try reading from local blob store first (works 100% offline!)
    let local_res = {
        let engine_guard = state.engine.read().await;
        if let Some(engine) = engine_guard.as_ref() {
            engine.store.blobs().get_bytes(parsed_hash).await.map(|b| b.to_vec()).ok()
        } else {
            let store_guard = state.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                store.blobs().get_bytes(parsed_hash).await.map(|b| b.to_vec()).ok()
            } else {
                None
            }
        }
    };

    let data = match local_res {
        Some(d) => d,
        None => {
            let blob_dir = match state.db.get_setting("custom_blob_dir") {
                Ok(Some(custom)) if !custom.trim().is_empty() => std::path::PathBuf::from(custom.trim()),
                _ => state.data_dir.join("blobs"),
            };
            let disk_file = blob_dir.join("data").join(format!("{hash}.data"));
            let fallback_disk_file = blob_dir.join(format!("{hash}.data"));
            let disk_bytes = if disk_file.exists() {
                tokio::fs::read(&disk_file).await.ok()
            } else if fallback_disk_file.exists() {
                tokio::fs::read(&fallback_disk_file).await.ok()
            } else {
                None
            };

            let verified_disk_bytes = if let Some(bytes) = disk_bytes {
                let actual = iroh_blobs::Hash::new(&bytes);
                if actual == parsed_hash {
                    let engine_guard = state.engine.read().await;
                    if let Some(engine) = engine_guard.as_ref() {
                        let _ = engine.store.blobs().add_bytes(bytes::Bytes::copy_from_slice(&bytes)).await;
                    }
                    Some(bytes)
                } else {
                    tracing::warn!(expected = %parsed_hash, actual = %actual, "Corrupted local disk blob file detected. Purging bad file.");
                    if disk_file.exists() {
                        let _ = tokio::fs::remove_file(&disk_file).await;
                    }
                    if fallback_disk_file.exists() {
                        let _ = tokio::fs::remove_file(&fallback_disk_file).await;
                    }
                    None
                }
            } else {
                None
            };

            if let Some(bytes) = verified_disk_bytes {
                bytes
            } else {
                // Blob is not in local cache: fetch on-demand from the swarm peers!
                let guard = state.engine.read().await;
                let engine = guard
                    .as_ref()
                    .ok_or_else(|| "File is not stored in local offline cache and peer network is currently disconnected.".to_string())?;

                let blob_rec = get_blob(&state.db, &hash).ok().flatten();
                let mut download_ok = false;
                let mut successful_seeder_id: Option<String> = None;

                // Track in resume queue before download starts
                let _ = crate::storage::blob_tracker::add_or_update_resume_queue(
                    &state.db,
                    &hash,
                    blob_rec.as_ref().and_then(|r| r.name.as_deref()),
                    blob_rec.as_ref().map(|r| r.size_bytes).unwrap_or(0),
                    0,
                    blob_rec.as_ref().and_then(|r| r.owner_node_id.as_deref()),
                );

                // Try the owner node first if known
                let my_node_id = engine.node_id().to_string();
                if let Some(rec) = &blob_rec {
                    if let Some(owner) = &rec.owner_node_id {
                        if !crate::storage::db::is_matching_node_id(owner, &my_node_id) {
                            if let Ok(owner_id) = owner.parse::<iroh::EndpointId>() {
                                let downloader = engine.store.downloader(&engine.endpoint);
                                if downloader.download(parsed_hash, vec![owner_id]).await.is_ok()
                                    && engine.store.blobs().get_bytes(parsed_hash).await.is_ok()
                                {
                                    download_ok = true;
                                    successful_seeder_id = Some(owner.clone());
                                }
                            }
                        }
                    }
                }

            // Fallback: try all other known seeders for this blob
            if !download_ok {
                let seeders: Vec<String> = {
                    let conn = state.db.lock();
                    let mut list = Vec::new();
                    if let Ok(mut stmt) = conn.prepare(
                        "SELECT peer_node_id FROM file_seeds WHERE hash = ?1 AND peer_node_id != ?2 ORDER BY last_seen_at DESC",
                    ) {
                        if let Ok(rows) = stmt.query_map(rusqlite::params![&hash, &my_node_id], |r| r.get(0)) {
                            for r in rows.flatten() {
                                list.push(r);
                            }
                        }
                    }
                    list
                };

                for seeder in seeders {
                    if let Ok(seeder_id) = seeder.parse::<iroh::EndpointId>() {
                        let downloader = engine.store.downloader(&engine.endpoint);
                        if downloader.download(parsed_hash, vec![seeder_id]).await.is_ok()
                            && engine.store.blobs().get_bytes(parsed_hash).await.is_ok()
                        {
                            download_ok = true;
                            successful_seeder_id = Some(seeder);
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

            let downloaded = engine
                .store
                .blobs()
                .get_bytes(parsed_hash)
                .await
                .map_err(|e| format!("blob read failed after swarm download: {e}"))?;

            // The blob now exists locally, so mark it present in the catalog.
            let _ = crate::storage::blob_tracker::mark_blob_local(&state.db, &hash);

            // Record dynamic network download throughput
            state.bandwidth.record_download(downloaded.len() as u64);

            // Remove from transfer resume queue on successful download completion
            let _ = crate::storage::blob_tracker::remove_resume_queue(&state.db, &hash);

            // Step 41: Bilateral seeding receipt issuance
            let my_node_id = engine.node_id().to_string();
            if let Some(ref seeder_id) = successful_seeder_id {
                if seeder_id != &my_node_id {
                    let now_unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;
                    let bytes_count = downloaded.len() as u64;

                    if let Ok(data_dir) = app.path().app_data_dir() {
                        if let Ok(key) = crate::p2p::identity::load_or_create_secret_key(&data_dir) {
                            let payload = crate::p2p::sync::create_receipt_payload(
                                &hash,
                                seeder_id,
                                &my_node_id,
                                bytes_count,
                                now_unix,
                            );
                            let signature_hex = crate::p2p::sync::sign_receipt(&key, &payload);
                            let _ = crate::storage::blob_tracker::store_seeding_receipt(
                                &state.db,
                                &hash,
                                seeder_id,
                                &my_node_id,
                                bytes_count,
                                now_unix,
                                &signature_hex,
                            );

                            broadcast_catalog_event(
                                &engine.endpoint,
                                &engine.sync_alpn,
                                SyncMessage::SeedingReceipt {
                                    hash: hash.clone(),
                                    seeder_node_id: seeder_id.clone(),
                                    recipient_node_id: my_node_id.clone(),
                                    bytes_transferred: bytes_count,
                                    issued_at: now_unix,
                                    signature_hex,
                                },
                            )
                            .await;
                        }
                    }
                }
            }

            // Broadcast to swarm that this node is now actively seeding this blob!
            broadcast_catalog_event(
                &engine.endpoint,
                &engine.sync_alpn,
                SyncMessage::FileSeeded {
                    hash: hash.clone(),
                    seeder_node_id: my_node_id,
                },
            )
            .await;

            downloaded.to_vec()
        }
    }
};

    if data.is_empty() {
        // An empty file is indistinguishable from the streamed large-file path
        // (both return an empty array), so explicitly signal completion.
        app.emit(
            "file-chunk-done",
            serde_json::json!({ "hash": hash, "total": 0u64 }),
        )
        .map_err(|e| format!("event emit failed: {e}"))?;
        return Ok(vec![]);
    }

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
        .and_then(|n| crate::storage::path_utils::sanitize_virtual_path(n).ok());

    let data = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    let size_bytes = data.len() as u64;

    let (store, owner_node_id, engine) = resolve_upload_context(&state).await?;

    let tag_info = store
        .blobs()
        .add_bytes(bytes::Bytes::from(data))
        .await
        .map_err(|e| format!("blob import failed: {e}"))?;

    let hash = tag_info.hash.to_hex().to_string();

    let _ = store
        .tags()
        .set(
            file_name.as_deref().unwrap_or(&hash).as_bytes(),
            tag_info,
        )
        .await;

    // If this hash previously had a deletion tombstone, clear it since the owner is intentionally re-uploading
    let _ = crate::storage::blob_tracker::remove_deletion_tombstone(&state.db, &hash);

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

    // Record initial seed for the local node
    let _ = record_peer_seed(&state.db, &hash, &owner_node_id);

    // Record upload bandwidth (once per successful upload)
    state.bandwidth.record_upload(size_bytes);

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
        delete_votes: Vec::new(),
        keep_votes: Vec::new(),
        deletion_scheduled_at: None,
        seed_count: 1,
        is_cached: true,
        ..Default::default()
    };
    broadcast_catalog_event_opt(&engine, SyncMessage::FileAdded(record)).await;

    tracing::info!(hash = %hash, name = ?file_name, owner = %owner_node_id, bytes = size_bytes, "Added file to workspace");
    Ok(hash)
}

/// Save edited note content, replacing the previous revision of the file.
///
/// Only the file's owner may save over it. If the content did not change the
/// original hash is returned. Otherwise the old revision is tombstoned (so it
/// cannot be resurrected by peers) and the new revision is published.
#[tauri::command]
pub async fn save_note(
    hash: String,
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let clean_name = crate::storage::path_utils::sanitize_virtual_path(&name)
        .map_err(|e| format!("Invalid filename: {e}"))?;
    let size_bytes = content.len() as u64;
    let (store, owner_node_id, engine) = resolve_upload_context(&state).await?;

    let tag_info = store
        .blobs()
        .add_bytes(bytes::Bytes::from(content.into_bytes()))
        .await
        .map_err(|e| format!("blob import failed: {e}"))?;
    let new_hash = tag_info.hash.to_hex().to_string();
    let _ = store.tags().set(clean_name.as_bytes(), tag_info).await;

    if new_hash == hash {
        let _ = touch_blob(&state.db, &hash);
        return Ok(hash);
    }

    // Verify the caller owns the existing file before replacing it.
    let old = get_blob(&state.db, &hash).map_err(|e| e.to_string())?;
    if let Some(rec) = &old {
        if let Some(owner) = rec.owner_node_id.as_deref() {
            if !crate::storage::db::is_matching_node_id(owner, &owner_node_id) {
                return Err("You can only edit notes that you own.".to_string());
            }
        }
    }

    let now = crate::storage::blob_tracker::now_unix();
    match engine.as_ref() {
        Some(engine) => {
            let tombstone = DeletionTombstone::create_signed(
                hash.clone(),
                owner_node_id.clone(),
                now,
                &engine.secret_key,
            );
            crate::storage::blob_tracker::record_deletion_tombstone(&state.db, &tombstone)
                .map_err(|e| e.to_string())?;
            let _ = store.tags().delete(hash.as_bytes()).await;
            broadcast_catalog_event(
                &engine.endpoint,
                &engine.sync_alpn,
                SyncMessage::FileDeletedTombstone(tombstone),
            )
            .await;
        }
        None => {
            let _ = delete_blob(&state.db, &hash);
            let _ = store.tags().delete(hash.as_bytes()).await;
        }
    }

    upsert_blob(
        &state.db,
        &new_hash,
        Some(&clean_name),
        size_bytes,
        false,
        Some(&owner_node_id),
    )
    .map_err(|e| e.to_string())?;
    let _ = record_peer_seed(&state.db, &new_hash, &owner_node_id);

    let record = BlobRecord {
        hash: new_hash.clone(),
        name: Some(clean_name.clone()),
        size_bytes,
        last_accessed_at: now,
        is_pinned: false,
        owner_node_id: Some(owner_node_id.clone()),
        delete_votes: Vec::new(),
        keep_votes: Vec::new(),
        deletion_scheduled_at: None,
        seed_count: 1,
        is_cached: true,
        ..Default::default()
    };
    broadcast_catalog_event_opt(&engine, SyncMessage::FileAdded(record)).await;

    tracing::info!(old_hash = %hash, new_hash = %new_hash, name = %clean_name, "Saved note revision");
    Ok(new_hash)
}

/// Upload a file to the shared cloud workspace via in-memory bytes (e.g. from UI file input).
#[tauri::command]
pub async fn upload_file_bytes(
    app: AppHandle,
    name: String,
    data: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Sanitize virtual path
    let clean_name = crate::storage::path_utils::sanitize_virtual_path(&name)
        .map_err(|e| format!("Invalid filename: {e}"))?;

    let size_bytes = data.len() as u64;
    let import_id = format!(
        "upload-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    emit_import_progress(
        Some(&app),
        &ImportProgressPayload {
            import_id: import_id.clone(),
            archive_name: clean_name.clone(),
            current_entry: 0,
            total_entries: 1,
            bytes_processed: 0,
            total_bytes: size_bytes,
            current_file: clean_name.clone(),
            bytes_per_sec: 0,
            status: "importing".to_string(),
            error: None,
        },
    );

    let start_time = std::time::Instant::now();

    let (store, owner_node_id, engine) = resolve_upload_context(&state).await?;

    let tag_info = match store
        .blobs()
        .add_bytes(bytes::Bytes::from(data))
        .await
    {
        Ok(t) => t,
        Err(e) => {
            let err_msg = format!("blob import failed: {e}");
            emit_import_progress(
                Some(&app),
                &ImportProgressPayload {
                    import_id: import_id.clone(),
                    archive_name: clean_name.clone(),
                    current_entry: 0,
                    total_entries: 1,
                    bytes_processed: 0,
                    total_bytes: size_bytes,
                    current_file: clean_name.clone(),
                    bytes_per_sec: 0,
                    status: "error".to_string(),
                    error: Some(err_msg.clone()),
                },
            );
            return Err(err_msg);
        }
    };

    let hash = tag_info.hash.to_hex().to_string();

    let _ = store
        .tags()
        .set(
            clean_name.as_bytes(),
            tag_info,
        )
        .await;

    // If this hash previously had a deletion tombstone, clear it since the owner is intentionally re-uploading
    let _ = crate::storage::blob_tracker::remove_deletion_tombstone(&state.db, &hash);

    // Record in SQLite with filename and owner node ID.
    upsert_blob(
        &state.db,
        &hash,
        Some(&clean_name),
        size_bytes,
        false,
        Some(&owner_node_id),
    )
    .map_err(|e| e.to_string())?;

    // Record initial seed for the local node
    let _ = record_peer_seed(&state.db, &hash, &owner_node_id);

    // Record upload bandwidth (once per successful upload)
    state.bandwidth.record_upload(size_bytes);

    // Broadcast file addition to the swarm
    let record = BlobRecord {
        hash: hash.clone(),
        name: Some(clean_name.clone()),
        size_bytes,
        last_accessed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        is_pinned: false,
        owner_node_id: Some(owner_node_id.clone()),
        delete_votes: Vec::new(),
        keep_votes: Vec::new(),
        deletion_scheduled_at: None,
        seed_count: 1,
        is_cached: true,
        ..Default::default()
    };
    broadcast_catalog_event_opt(&engine, SyncMessage::FileAdded(record)).await;

    let elapsed = start_time.elapsed().as_secs_f64();
    let speed = if elapsed > 0.02 {
        (size_bytes as f64 / elapsed) as u64
    } else {
        0
    };

    emit_import_progress(
        Some(&app),
        &ImportProgressPayload {
            import_id,
            archive_name: clean_name.clone(),
            current_entry: 1,
            total_entries: 1,
            bytes_processed: size_bytes,
            total_bytes: size_bytes,
            current_file: clean_name.clone(),
            bytes_per_sec: speed,
            status: "completed".to_string(),
            error: None,
        },
    );

    tracing::info!(hash = %hash, name = %clean_name, owner = %owner_node_id, bytes = size_bytes, "Uploaded file to workspace");
    Ok(hash)
}

/// Delete a file by hash.
/// ONLY the original uploader who owns the file can delete it unilaterally.
/// Non-owners are prevented from deleting other students' files.
#[tauri::command]
pub async fn delete_file(
    hash: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let blob = get_blob(&state.db, &hash)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("file '{hash}' not found in workspace"))?;

    let (my_node_id, engine, secret_key) = resolve_identity_context(&state).await?;

    let is_owner = blob
        .owner_node_id
        .as_deref()
        .map(|o| crate::storage::db::is_matching_node_id(o, &my_node_id))
        .unwrap_or(false);

    if !is_owner {
        return Err(
            "Permission denied: You can only directly delete files you uploaded. For community or orphaned files, use 'Vote to Delete' to mark them for removal.".to_string(),
        );
    }

    // 1. Create a cryptographically signed DeletionTombstone proving the owner authorized deletion
    let now = crate::storage::blob_tracker::now_unix();
    let tombstone = crate::storage::blob_tracker::DeletionTombstone::create_signed(
        hash.clone(),
        my_node_id.clone(),
        now,
        &secret_key,
    );

    // 2. Record tombstone in SQLite (which removes local_blobs and purges physical disk files)
    crate::storage::blob_tracker::record_deletion_tombstone(&state.db, &tombstone)
        .map_err(|e| e.to_string())?;

    // 3. Delete blob data tags from local Iroh store
    if let Some(store) = resolve_local_store(&state).await {
        let _ = store.tags().delete(hash.as_bytes()).await;
    }

    // 4. Broadcast cryptographically signed tombstone deletion event to the entire swarm
    broadcast_catalog_event_opt(&engine, SyncMessage::FileDeletedTombstone(tombstone)).await;

    tracing::info!(hash = %hash, "Deleted file from workspace (owner signed tombstone)");
    Ok(())
}

/// Cast a vote to mark a file for deletion.
/// Deletion requires >= 3 delete votes AND >= 86% delete consensus.
/// Even upon reaching this ratio, a 24-hour grace period begins, allowing peers to defend/keep the file.
#[tauri::command]
pub async fn vote_delete_file(
    hash: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let _blob = get_blob(&state.db, &hash)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("file '{hash}' not found in workspace"))?;

    let (my_node_id, engine, _) = resolve_identity_context(&state).await?;

    let votes = cast_delete_vote(&state.db, &hash, &my_node_id).map_err(|e| e.to_string())?;

    // Broadcast vote to all connected swarm peers
    broadcast_catalog_event_opt(
        &engine,
        SyncMessage::FileVoteDelete {
            hash: hash.clone(),
            voter_node_id: my_node_id.clone(),
        },
    )
    .await;

    tracing::info!(hash = %hash, voter = %my_node_id, total_delete_votes = votes, "Voted to delete file");
    Ok(votes)
}

/// Retract a previously cast delete vote.
#[tauri::command]
pub async fn unvote_delete_file(
    hash: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let (my_node_id, engine, _) = resolve_identity_context(&state).await?;

    let votes = remove_delete_vote(&state.db, &hash, &my_node_id).map_err(|e| e.to_string())?;

    // Broadcast unvote to all swarm peers
    broadcast_catalog_event_opt(
        &engine,
        SyncMessage::FileUnvoteDelete {
            hash: hash.clone(),
            voter_node_id: my_node_id,
        },
    )
    .await;

    Ok(votes)
}

/// Cast a vote to KEEP / DEFEND a file against deletion.
/// Keep votes increase the defense quorum and will abort a scheduled deletion if ratio falls below 86%.
#[tauri::command]
pub async fn vote_keep_file(
    hash: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let _blob = get_blob(&state.db, &hash)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("file '{hash}' not found in workspace"))?;

    let (my_node_id, engine, _) = resolve_identity_context(&state).await?;

    let votes = cast_keep_vote(&state.db, &hash, &my_node_id).map_err(|e| e.to_string())?;

    // Broadcast keep vote to all connected swarm peers
    broadcast_catalog_event_opt(
        &engine,
        SyncMessage::FileVoteKeep {
            hash: hash.clone(),
            voter_node_id: my_node_id.clone(),
        },
    )
    .await;

    tracing::info!(hash = %hash, voter = %my_node_id, total_keep_votes = votes, "Voted to keep/defend file");
    Ok(votes)
}

/// Retract a previously cast keep vote.
#[tauri::command]
pub async fn unvote_keep_file(
    hash: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let (my_node_id, engine, _) = resolve_identity_context(&state).await?;

    let votes = remove_keep_vote(&state.db, &hash, &my_node_id).map_err(|e| e.to_string())?;

    // Broadcast unvote to all swarm peers
    broadcast_catalog_event_opt(
        &engine,
        SyncMessage::FileUnvoteKeep {
            hash: hash.clone(),
            voter_node_id: my_node_id,
        },
    )
    .await;

    Ok(votes)
}

/// Evict a blob from the local device's cache without deleting it from the cloud catalog.
/// PREVENTED if the local node is the only active seed in the swarm to avoid irrecoverable data loss.
#[tauri::command]
pub async fn evict_local_file(hash: String, state: State<'_, AppState>) -> Result<(), String> {
    let (my_node_id, engine, _) = resolve_identity_context(&state).await?;

    // User requirement: "don't allow removing from disc if you're the only active seed"
    let remote_seeds = get_seed_count(&state.db, &hash, Some(&my_node_id), false).map_err(|e| e.to_string())?;
    if remote_seeds == 0 {
        return Err(
            "Cannot remove file from disk: You are the ONLY active seed in the swarm! Deleting your copy would permanently destroy this file for all students. Wait until another peer caches it, or vote to delete the file if it is no longer needed."
                .to_string(),
        );
    }

    if let Some(store) = resolve_local_store(&state).await {
        let _ = store.tags().delete(hash.as_bytes()).await;
    }

    // Broadcast to the swarm that this node is no longer seeding this blob
    broadcast_catalog_event_opt(
        &engine,
        SyncMessage::FileUnseeded {
            hash: hash.clone(),
            seeder_node_id: my_node_id,
        },
    )
    .await;

    Ok(())
}

/// Move or rename a file to a new workspace path.
#[tauri::command]
pub async fn move_file(
    hash: String,
    new_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let clean_path = crate::storage::path_utils::sanitize_virtual_path(&new_path)
        .map_err(|e| format!("Invalid destination path: {e}"))?;

    let blob = get_blob(&state.db, &hash)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("file '{hash}' not found in workspace"))?;

    let (my_node_id, engine, _) = resolve_identity_context(&state).await?;

    // SECURITY: Only the file's owner can move or rename it across the workspace
    let is_owner = blob
        .owner_node_id
        .as_deref()
        .map(|o| crate::storage::db::is_matching_node_id(o, &my_node_id))
        .unwrap_or(true);

    if !is_owner {
        return Err("Permission denied: You can only move or rename files you uploaded. For community files, organize them into folders using bookmarks or local copies.".to_string());
    }

    rename_blob(&state.db, &hash, &clean_path).map_err(|e| e.to_string())?;

    // Broadcast file moved event to connected peers
    broadcast_catalog_event_opt(
        &engine,
        SyncMessage::FileMoved {
            hash,
            new_name: clean_path,
        },
    )
    .await;

    Ok(())
}

/// Save a blob to a specified local destination file path.
///
/// `dest_path` is expected to be an absolute path chosen by the native save
/// dialog. Relative paths are resolved under the OS Downloads directory.
#[tauri::command]
pub async fn export_file(
    hash: String,
    dest_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let dest_path = dest_path.trim();
    if dest_path.is_empty() {
        return Err("Destination path is empty".to_string());
    }
    if dest_path.contains('\0') {
        return Err("Invalid destination path: contains null byte".to_string());
    }

    let parsed_hash = hash
        .parse::<iroh_blobs::Hash>()
        .map_err(|e| format!("invalid hash '{hash}': {e}"))?;

    if crate::storage::blob_tracker::is_quarantined(&state.db, &hash).unwrap_or(false) {
        return Err("Security restriction: Exporting this file is blocked because it has been quarantined by community safety reports.".to_string());
    }
    if crate::storage::blob_tracker::is_blob_blocked(&state.db, &hash).unwrap_or(false) {
        return Err("Security restriction: Exporting this file is blocked because it is on your local blocklist.".to_string());
    }

    // Read from the running engine if available, otherwise the preloaded store.
    let data = {
        let engine_guard = state.engine.read().await;
        if let Some(engine) = engine_guard.as_ref() {
            engine
                .store
                .blobs()
                .get_bytes(parsed_hash)
                .await
                .map(|b| b.to_vec())
                .ok()
        } else {
            drop(engine_guard);
            let store_guard = state.store.read().await;
            match store_guard.as_ref() {
                Some(store) => store
                    .blobs()
                    .get_bytes(parsed_hash)
                    .await
                    .map(|b| b.to_vec())
                    .ok(),
                None => None,
            }
        }
    }
    .ok_or_else(|| {
        "File is not cached locally. Open or download it once before exporting.".to_string()
    })?;

    let requested = std::path::PathBuf::from(dest_path);
    let target = if requested.is_absolute() {
        requested
    } else {
        app.path()
            .download_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(requested)
    };

    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!("failed to create destination directory '{}': {e}", parent.display())
            })?;
        }
    }

    if is_executable_masquerading_as_doc(&target, &data) {
        return Err("Security alert: This file claims to be a document or image, but contains executable binary headers (masquerading attack). Export blocked.".to_string());
    }

    std::fs::write(&target, data)
        .map_err(|e| format!("failed to write destination file '{}': {e}", target.display()))?;

    attach_windows_zone_identifier(&target);

    Ok(())
}

/// Read a blob from the running engine, the preloaded store, or local disk fallback.
async fn load_blob_bytes(state: &AppState, hash: iroh_blobs::Hash) -> Option<Vec<u8>> {
    {
        let engine_guard = state.engine.read().await;
        if let Some(engine) = engine_guard.as_ref() {
            if let Ok(bytes) = engine.store.blobs().get_bytes(hash).await {
                return Some(bytes.to_vec());
            }
        }
    }
    {
        let store_guard = state.store.read().await;
        if let Some(store) = store_guard.as_ref() {
            if let Ok(bytes) = store.blobs().get_bytes(hash).await {
                return Some(bytes.to_vec());
            }
        }
    }

    // Disk fallback: check data_dir and blob_dir on disk if redb store missed it
    let hash_hex = hash.to_hex().to_string();
    let blob_dir = match state.db.get_setting("custom_blob_dir") {
        Ok(Some(custom)) if !custom.trim().is_empty() => std::path::PathBuf::from(custom.trim()),
        _ => state.data_dir.join("blobs"),
    };
    let disk_file = blob_dir.join("data").join(format!("{hash_hex}.data"));
    let fallback_disk_file = blob_dir.join(format!("{hash_hex}.data"));
    let disk_bytes = if disk_file.exists() {
        tokio::fs::read(&disk_file).await.ok()
    } else if fallback_disk_file.exists() {
        tokio::fs::read(&fallback_disk_file).await.ok()
    } else {
        None
    };

    if let Some(bytes) = disk_bytes {
        // Auto-heal / re-index into store
        let engine_guard = state.engine.read().await;
        if let Some(engine) = engine_guard.as_ref() {
            let _ = engine.store.blobs().add_bytes(bytes::Bytes::copy_from_slice(&bytes)).await;
        }
        let _ = crate::storage::blob_tracker::mark_blob_local(&state.db, &hash_hex);
        return Some(bytes);
    }

    None
}

/// Fetch a blob from known peers (owner first, then active seeders).
async fn fetch_blob_from_peers(
    state: &AppState,
    hash: &str,
    parsed_hash: iroh_blobs::Hash,
) -> Result<Vec<u8>, String> {
    let engine_guard = state.engine.read().await;
    let engine = engine_guard.as_ref().ok_or_else(|| {
        "File is not stored in local offline cache and the peer network is currently disconnected."
            .to_string()
    })?;
    let my_node_id = engine.node_id().to_string();
    let blob_rec = get_blob(&state.db, hash).ok().flatten();

    if let Some(owner) = blob_rec.as_ref().and_then(|r| r.owner_node_id.clone()) {
        if !crate::storage::db::is_matching_node_id(&owner, &my_node_id) {
            if let Ok(owner_id) = owner.parse::<iroh::EndpointId>() {
                let downloader = engine.store.downloader(&engine.endpoint);
                if downloader
                    .download(parsed_hash, vec![owner_id])
                    .await
                    .is_ok()
                {
                    if let Ok(bytes) = engine.store.blobs().get_bytes(parsed_hash).await {
                        let actual_hash = iroh_blobs::Hash::new(&bytes);
                        if actual_hash != parsed_hash {
                            tracing::error!(expected = %parsed_hash, actual = %actual_hash, "Corrupted/poisoned chunk received from peer!");
                            return Err("Data integrity failure: downloaded payload does not match expected BLAKE3 hash.".to_string());
                        }
                        let _ = crate::storage::blob_tracker::mark_blob_local(&state.db, hash);
                        return Ok(bytes.to_vec());
                    }
                }
            }
        }
    }

    let seeders: Vec<String> = {
        let conn = state.db.lock();
        let mut list = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT peer_node_id FROM file_seeds WHERE hash = ?1 AND peer_node_id != ?2 ORDER BY last_seen_at DESC",
        ) {
            if let Ok(rows) = stmt.query_map(rusqlite::params![hash, &my_node_id], |r| r.get(0)) {
                for row in rows.flatten() {
                    list.push(row);
                }
            }
        }
        list
    };

    for seeder in seeders {
        if let Ok(seeder_id) = seeder.parse::<iroh::EndpointId>() {
            let downloader = engine.store.downloader(&engine.endpoint);
            if downloader
                .download(parsed_hash, vec![seeder_id])
                .await
                .is_ok()
            {
                if let Ok(bytes) = engine.store.blobs().get_bytes(parsed_hash).await {
                    let actual_hash = iroh_blobs::Hash::new(&bytes);
                    if actual_hash != parsed_hash {
                        tracing::error!(expected = %parsed_hash, actual = %actual_hash, "Corrupted/poisoned chunk received from peer!");
                        continue;
                    }
                    let _ = crate::storage::blob_tracker::mark_blob_local(&state.db, hash);
                    return Ok(bytes.to_vec());
                }
            }
        }
    }

    let file_display = blob_rec
        .as_ref()
        .and_then(|r| r.name.as_deref())
        .unwrap_or(hash);

    let is_owner = if let Some(owner) = blob_rec.as_ref().and_then(|r| r.owner_node_id.as_deref()) {
        crate::storage::db::is_matching_node_id(owner, &my_node_id)
    } else {
        false
    };

    if is_owner {
        Err(format!(
            "Local file copy for \"{file_display}\" is missing from disk storage, and no other online peers currently have a copy to restore it."
        ))
    } else {
        Err(format!(
            "File \"{file_display}\" is not cached locally and no online peers are currently available to serve it."
        ))
    }
}

/// Load a blob from local storage, falling back to an on-demand swarm fetch.
async fn load_or_fetch_blob_bytes(state: &AppState, hash: &str) -> Result<Vec<u8>, String> {
    let _ = crate::storage::blob_tracker::touch_blob(&state.db, hash);
    let parsed_hash = hash
        .parse::<iroh_blobs::Hash>()
        .map_err(|e| format!("invalid hash '{hash}': {e}"))?;
    if let Some(bytes) = load_blob_bytes(state, parsed_hash).await {
        return Ok(bytes);
    }
    fetch_blob_from_peers(state, hash, parsed_hash).await
}

/// Reduce a user-supplied file name to a safe bare file name.
fn safe_file_name(raw: &str, fallback: &str) -> String {
    let candidate = std::path::Path::new(raw.trim())
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let cleaned: String = candidate
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    if cleaned.trim().is_empty() || cleaned == "." || cleaned == ".." {
        fallback.to_string()
    } else {
        cleaned
    }
}

/// Choose a non-colliding destination path by appending " (n)" before the extension.
fn unique_destination(dir: &std::path::Path, file_name: &str) -> std::path::PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let path = std::path::Path::new(file_name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy()));
    for index in 1..10_000 {
        let name = match &ext {
            Some(ext) => format!("{stem} ({index}){ext}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(file_name)
}

/// Prompt a native save dialog for exporting a single catalog file.
#[tauri::command]
pub async fn pick_file_save_dialog(default_name: Option<String>) -> Result<Option<String>, String> {
    let name = safe_file_name(
        default_name.as_deref().unwrap_or("p2pstud-file.bin"),
        "p2pstud-file.bin",
    );
    let file = rfd::AsyncFileDialog::new()
        .set_file_name(&name)
        .set_title("Save File Copy")
        .save_file()
        .await;
    Ok(file.map(|h| h.path().to_string_lossy().to_string()))
}

/// Save a copy of a catalog file directly into the user's Downloads folder.
#[tauri::command]
pub async fn download_file_to_downloads(
    hash: String,
    file_name: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if crate::storage::blob_tracker::is_quarantined(&state.db, &hash).unwrap_or(false) {
        return Err("Security restriction: Downloading this file is blocked because it has been quarantined by community safety reports.".to_string());
    }
    if crate::storage::blob_tracker::is_blob_blocked(&state.db, &hash).unwrap_or(false) {
        return Err("Security restriction: Downloading this file is blocked because it is on your local blocklist.".to_string());
    }

    let data = load_or_fetch_blob_bytes(&state, &hash).await?;

    let downloads = app
        .path()
        .download_dir()
        .map_err(|e| format!("Failed to resolve Downloads folder: {e}"))?;
    std::fs::create_dir_all(&downloads)
        .map_err(|e| format!("Failed to create Downloads folder: {e}"))?;

    let fallback = format!("{}-{}", &hash[..hash.len().min(8)], "download");
    let name = safe_file_name(file_name.as_deref().unwrap_or(&fallback), &fallback);
    let target = unique_destination(&downloads, &name);

    if is_executable_masquerading_as_doc(&target, &data) {
        return Err("Security alert: This file claims to be a document or image, but contains executable binary headers (masquerading attack). Download blocked.".to_string());
    }

    std::fs::write(&target, data)
        .map_err(|e| format!("Failed to save '{}': {e}", target.display()))?;
    attach_windows_zone_identifier(&target);
    Ok(target.to_string_lossy().to_string())
}

/// Attach Mark-of-the-Web (Zone.Identifier) on Windows so downloaded/materialized files
/// are properly inspected and sandboxed by Windows Defender, SmartScreen, and Office Protected View.
fn attach_windows_zone_identifier(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        let zone_path = format!("{}:Zone.Identifier", path.display());
        let _ = std::fs::write(&zone_path, "[ZoneTransfer]\r\nZoneId=3\r\n");
    }
    #[cfg(not(target_os = "windows"))]
    let _ = path;
}

/// Check if a target file has an executable, macro, or potentially hazardous script extension.
fn is_dangerous_file_extension(path: &std::path::Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let lower = ext.to_ascii_lowercase();
    const BLOCKED_EXTENSIONS: &[&str] = &[
        "exe", "bat", "cmd", "com", "scr", "msi", "msp", "vbs", "vbe", "js", "jse", "wsf",
        "wsh", "ps1", "ps1xml", "ps2", "ps2xml", "psc1", "psc2", "msh", "msh1", "msh2",
        "mshxml", "msh1xml", "msh2xml", "inf", "reg", "hta", "cpl", "jar", "pif", "scf",
        "lnk", "iso", "img", "vhd", "vhdx", "dll", "sys", "drv", "bin",
        // Dangerous web and markup formats that execute active code when opened in browser
        "html", "htm", "xhtml", "svg",
        // Dangerous macro-enabled Office document formats
        "docm", "xlsm", "pptm", "dotm", "xltm", "potm",
    ];
    BLOCKED_EXTENSIONS.contains(&lower.as_str())
}

/// Check if file content contains executable binary headers (PE/MZ, ELF, Mach-O)
/// masquerading under harmless document, image, or text extensions.
fn is_executable_masquerading_as_doc(path: &std::path::Path, data: &[u8]) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let lower = ext.to_ascii_lowercase();
    const DOC_EXTENSIONS: &[&str] = &[
        "pdf", "png", "jpg", "jpeg", "gif", "webp", "bmp", "tiff",
        "txt", "md", "csv", "json", "docx", "xlsx", "pptx", "odt",
        "mp3", "mp4", "wav", "flac", "ogg", "epub",
    ];
    if !DOC_EXTENSIONS.contains(&lower.as_str()) {
        return false;
    }

    // Windows PE / DOS binary: starts with "MZ" (0x4D, 0x5A)
    if data.len() >= 2 && &data[0..2] == b"MZ" {
        return true;
    }
    // Linux ELF binary: starts with 0x7F, 'E', 'L', 'F'
    if data.len() >= 4 && &data[0..4] == b"\x7fELF" {
        return true;
    }
    // macOS Mach-O binary signatures
    if data.len() >= 4 && (
        &data[0..4] == b"\xfe\xed\xfa\xce" ||
        &data[0..4] == b"\xfe\xed\xfa\xcf" ||
        &data[0..4] == b"\xce\xfa\xed\xfe" ||
        &data[0..4] == b"\xcf\xfa\xed\xfe" ||
        &data[0..4] == b"\xca\xfe\xba\xbe"
    ) {
        return true;
    }
    false
}

/// Materialize a catalog file on disk and open it with the OS default application.
#[tauri::command]
pub async fn open_file_with_default_app(
    hash: String,
    file_name: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if crate::storage::blob_tracker::is_quarantined(&state.db, &hash).unwrap_or(false) {
        return Err("Security restriction: Opening this file is blocked because it has been quarantined by community safety reports.".to_string());
    }
    if crate::storage::blob_tracker::is_blob_blocked(&state.db, &hash).unwrap_or(false) {
        return Err("Security restriction: Opening this file is blocked because it is on your local blocklist.".to_string());
    }

    let data = load_or_fetch_blob_bytes(&state, &hash).await?;

    let preview_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("preview");
    std::fs::create_dir_all(&preview_dir)
        .map_err(|e| format!("Failed to create preview cache: {e}"))?;

    let fallback = format!("{}-{}", &hash[..hash.len().min(8)], "file");
    let name = safe_file_name(file_name.as_deref().unwrap_or(&fallback), &fallback);
    let target = preview_dir.join(&name);

    // SECURITY: Prevent arbitrary code execution from malicious shared files.
    if is_dangerous_file_extension(&target) {
        return Err(
            "Security restriction: Opening executable, script, or binary files directly with the system viewer is blocked. Please download or save the file to inspect it safely."
                .to_string(),
        );
    }

    // SECURITY: Detect executable binary masquerading as document/image
    if is_executable_masquerading_as_doc(&target, &data) {
        return Err("Security alert: This file claims to be a document or image, but contains executable binary headers (masquerading attack). Opening blocked.".to_string());
    }

    std::fs::write(&target, data)
        .map_err(|e| format!("Failed to write preview file '{}': {e}", target.display()))?;
    attach_windows_zone_identifier(&target);

    let target_str = target.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    let mut opener = {
        // Use rundll32 FileProtocolHandler to invoke default shell association without running cmd.exe interpreter
        let mut cmd = std::process::Command::new("rundll32.exe");
        cmd.args(["url.dll,FileProtocolHandler", &target_str]);
        cmd
    };
    #[cfg(target_os = "macos")]
    let mut opener = {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(&target_str);
        cmd
    };
    #[cfg(target_os = "linux")]
    let mut opener = {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(&target_str);
        cmd
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let mut opener = std::process::Command::new("true");

    opener
        .spawn()
        .map_err(|e| format!("Failed to open file with the system viewer: {e}"))?;
    Ok(target_str)
}


/// Summary of files imported from a ZIP archive.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ZipImportSummary {
    pub files_imported: usize,
    pub total_bytes: u64,
    pub errors: Vec<String>,
}

/// File metadata returned by native file selection dialogs.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SelectedFileInfo {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
}

/// Native file picker dialog to pick one or more files without loading them into browser memory.
#[tauri::command]
pub async fn pick_files_dialog() -> Result<Option<Vec<SelectedFileInfo>>, String> {
    let files = rfd::AsyncFileDialog::new()
        .set_title("Select Files to Upload")
        .pick_files()
        .await;

    if let Some(handles) = files {
        let mut list = Vec::new();
        for handle in handles {
            let path = handle.path().to_path_buf();
            let meta = std::fs::metadata(&path).map_err(|e| format!("cannot read file metadata: {e}"))?;
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            list.push(SelectedFileInfo {
                path: path.to_string_lossy().to_string(),
                name,
                size_bytes: meta.len(),
            });
        }
        Ok(Some(list))
    } else {
        Ok(None)
    }
}

/// Native file picker dialog to pick a ZIP file without loading it into browser memory.
#[tauri::command]
pub async fn pick_zip_dialog() -> Result<Option<SelectedFileInfo>, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("ZIP Archives", &["zip"])
        .set_title("Select ZIP Archive to Import")
        .pick_file()
        .await;

    if let Some(handle) = file {
        let path = handle.path().to_path_buf();
        let meta = std::fs::metadata(&path).map_err(|e| format!("cannot read file metadata: {e}"))?;
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        Ok(Some(SelectedFileInfo {
            path: path.to_string_lossy().to_string(),
            name,
            size_bytes: meta.len(),
        }))
    } else {
        Ok(None)
    }
}

/// Generic helper to unpack and import any seekable ZIP archive into Iroh store & SQLite.
async fn import_zip_archive_internal<R: std::io::Read + std::io::Seek>(
    mut archive: zip::ZipArchive<R>,
    archive_name: &str,
    target_folder: Option<String>,
    state: &AppState,
    app: Option<&AppHandle>,
) -> Result<ZipImportSummary, String> {
    let (store, owner_node_id, engine) = resolve_upload_context(state).await?;

    let mut summary = ZipImportSummary {
        files_imported: 0,
        total_bytes: 0,
        errors: Vec::new(),
    };

    let target_prefix = match target_folder {
        Some(f) => {
            let trimmed = f.trim().trim_matches('/');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        None => None,
    };

    const MAX_ZIP_ENTRY_SIZE: u64 = 500 * 1024 * 1024; // 500 MB limit per entry
    const MAX_TOTAL_UNCOMPRESSED: u64 = 5 * 1024 * 1024 * 1024; // 5 GB total limit

    let total_entries = archive.len();
    let mut total_uncompressed_bytes: u64 = 0;
    for idx in 0..total_entries {
        if let Ok(entry) = archive.by_index(idx) {
            if !entry.is_dir() {
                total_uncompressed_bytes += entry.size();
                if entry.size() > MAX_ZIP_ENTRY_SIZE || total_uncompressed_bytes > MAX_TOTAL_UNCOMPRESSED {
                    summary.errors.push("ZIP archive exceeds safe decompression limit (zip bomb protection)".to_string());
                    return Ok(summary);
                }
            }
        }
    }

    let import_id = format!(
        "zip-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let mut last_emit_time = std::time::Instant::now();
    let mut last_emit_bytes = 0u64;
    let mut rolling_bps = 0u64;

    emit_import_progress(
        app,
        &ImportProgressPayload {
            import_id: import_id.clone(),
            archive_name: archive_name.to_string(),
            current_entry: 0,
            total_entries,
            bytes_processed: 0,
            total_bytes: total_uncompressed_bytes,
            current_file: "Starting extraction...".to_string(),
            bytes_per_sec: 0,
            status: "importing".to_string(),
            error: None,
        },
    );

    for i in 0..total_entries {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(e) => {
                summary.errors.push(format!("entry {i} read error: {e}"));
                continue;
            }
        };

        let entry_name = match file.name() {
            Ok(n) => n.to_string(),
            Err(e) => {
                summary.errors.push(format!("entry {i} has invalid name/header: {e}"));
                continue;
            }
        };

        if file.is_dir() {
            let enclosed = entry_name.trim_end_matches('/');
            let raw = if let Some(ref prefix) = target_prefix {
                format!("{prefix}/{enclosed}")
            } else {
                enclosed.to_string()
            };
            if let Ok(clean) = crate::storage::path_utils::sanitize_virtual_path(&raw) {
                let _ = crate::storage::blob_tracker::create_virtual_folder(&state.db, &clean);
            }
            continue;
        }

        // Security check: Zip Slip prevention
        let enclosed = match file.enclosed_name() {
            Some(path) => path.to_string_lossy().replace('\\', "/"),
            None => {
                summary.errors.push(format!("Zip Slip directory traversal detected for entry {entry_name}"));
                continue;
            }
        };

        let raw_virtual_path = if let Some(ref prefix) = target_prefix {
            format!("{}/{}", prefix, enclosed)
        } else {
            enclosed
        };

        let sanitized_path = match crate::storage::path_utils::sanitize_virtual_path(&raw_virtual_path) {
            Ok(p) => p,
            Err(e) => {
                summary.errors.push(format!("Invalid path for entry '{entry_name}': {e}"));
                continue;
            }
        };

        // Auto-create parent folders in SQLite
        if let Some((parent, _)) = sanitized_path.rsplit_once('/') {
            let _ = crate::storage::blob_tracker::create_virtual_folder(&state.db, parent);
        }

        // Bounded read using std::io::Read::take to defend against deceptive uncompressed headers
        use std::io::Read;
        let mut file_bytes = Vec::with_capacity((file.size().min(16 * 1024 * 1024)) as usize);
        if let Err(e) = (&mut file).take(MAX_ZIP_ENTRY_SIZE).read_to_end(&mut file_bytes) {
            summary.errors.push(format!("Failed reading bytes for entry '{sanitized_path}': {e}"));
            continue;
        }

        let size_bytes = file_bytes.len() as u64;

        let tag_info = match store
            .blobs()
            .add_bytes(bytes::Bytes::from(file_bytes))
            .await
        {
            Ok(t) => t,
            Err(e) => {
                summary.errors.push(format!("Failed importing '{sanitized_path}' to store: {e}"));
                continue;
            }
        };

        let hash = tag_info.hash.to_hex().to_string();

        let _ = store
            .tags()
            .set(sanitized_path.as_bytes(), tag_info)
            .await;

        let _ = crate::storage::blob_tracker::remove_deletion_tombstone(&state.db, &hash);

        if let Err(e) = upsert_blob(
            &state.db,
            &hash,
            Some(&sanitized_path),
            size_bytes,
            false,
            Some(&owner_node_id),
        ) {
            summary.errors.push(format!("Database insert failed for '{sanitized_path}': {e}"));
            continue;
        }

        let _ = record_peer_seed(&state.db, &hash, &owner_node_id);

        let record = BlobRecord {
            hash: hash.clone(),
            name: Some(sanitized_path.clone()),
            size_bytes,
            last_accessed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            is_pinned: false,
            owner_node_id: Some(owner_node_id.clone()),
            delete_votes: Vec::new(),
            keep_votes: Vec::new(),
            deletion_scheduled_at: None,
            seed_count: 1,
            is_cached: true,
            ..Default::default()
        };

        broadcast_catalog_event_opt(&engine, SyncMessage::FileAdded(record)).await;

        summary.files_imported += 1;
        summary.total_bytes += size_bytes;
        state.bandwidth.record_upload(size_bytes);

        let now = std::time::Instant::now();
        let dt = now.duration_since(last_emit_time).as_secs_f64();
        if dt >= 0.08 || i + 1 == total_entries {
            let delta_bytes = summary.total_bytes.saturating_sub(last_emit_bytes);
            if dt > 0.01 {
                let inst_bps = (delta_bytes as f64 / dt) as u64;
                rolling_bps = if rolling_bps == 0 {
                    inst_bps
                } else {
                    ((rolling_bps as f64 * 0.35) + (inst_bps as f64 * 0.65)) as u64
                };
            }
            last_emit_time = now;
            last_emit_bytes = summary.total_bytes;

            emit_import_progress(
                app,
                &ImportProgressPayload {
                    import_id: import_id.clone(),
                    archive_name: archive_name.to_string(),
                    current_entry: i + 1,
                    total_entries,
                    bytes_processed: summary.total_bytes,
                    total_bytes: total_uncompressed_bytes.max(summary.total_bytes),
                    current_file: entry_name.clone(),
                    bytes_per_sec: rolling_bps,
                    status: "importing".to_string(),
                    error: None,
                },
            );
        }
    }

    emit_import_progress(
        app,
        &ImportProgressPayload {
            import_id: import_id.clone(),
            archive_name: archive_name.to_string(),
            current_entry: total_entries,
            total_entries,
            bytes_processed: summary.total_bytes,
            total_bytes: total_uncompressed_bytes.max(summary.total_bytes),
            current_file: "Completed".to_string(),
            bytes_per_sec: 0,
            status: "completed".to_string(),
            error: None,
        },
    );

    tracing::info!(
        imported = summary.files_imported,
        bytes = summary.total_bytes,
        "Finished ZIP archive import"
    );

    Ok(summary)
}

/// Unpack and import in-memory ZIP archive bytes (e.g. from web drag-and-drop).
#[tauri::command]
pub async fn import_zip_bytes(
    app: AppHandle,
    data: Vec<u8>,
    target_folder: Option<String>,
    state: State<'_, AppState>,
) -> Result<ZipImportSummary, String> {
    let cursor = std::io::Cursor::new(data);
    let archive = zip::ZipArchive::new(cursor).map_err(|e| format!("invalid zip archive: {e}"))?;
    import_zip_archive_internal(archive, "Archive.zip", target_folder, &state, Some(&app)).await
}

/// Import a local ZIP archive file directly from disk via streaming BufReader (supports multi-GB ZIPs).
#[tauri::command]
pub async fn import_zip_file(
    app: AppHandle,
    file_path: String,
    target_folder: Option<String>,
    state: State<'_, AppState>,
) -> Result<ZipImportSummary, String> {
    let path = std::path::Path::new(&file_path);
    let archive_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Archive.zip".to_string());
    let file = std::fs::File::open(path).map_err(|e| format!("cannot open zip file '{file_path}': {e}"))?;
    let reader = std::io::BufReader::new(file);
    let archive = zip::ZipArchive::new(reader).map_err(|e| format!("invalid zip archive: {e}"))?;
    import_zip_archive_internal(archive, &archive_name, target_folder, &state, Some(&app)).await
}

/// Upload a file directly from a disk path (supports multi-GB files without memory exhaustion).
#[tauri::command]
pub async fn upload_file_path(
    app: AppHandle,
    file_path: String,
    target_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = std::path::Path::new(&file_path);
    let meta = std::fs::metadata(path).map_err(|e| format!("cannot stat file '{file_path}': {e}"))?;
    if !meta.is_file() {
        return Err(format!("'{file_path}' is not a regular file"));
    }
    let size_bytes = meta.len();

    let raw_name = target_name.unwrap_or_else(|| {
        path.file_name().unwrap_or_default().to_string_lossy().to_string()
    });
    let clean_name = crate::storage::path_utils::sanitize_virtual_path(&raw_name)
        .map_err(|e| format!("Invalid filename: {e}"))?;

    let import_id = format!(
        "upload-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    emit_import_progress(
        Some(&app),
        &ImportProgressPayload {
            import_id: import_id.clone(),
            archive_name: clean_name.clone(),
            current_entry: 0,
            total_entries: 1,
            bytes_processed: 0,
            total_bytes: size_bytes,
            current_file: clean_name.clone(),
            bytes_per_sec: 0,
            status: "importing".to_string(),
            error: None,
        },
    );

    let start_time = std::time::Instant::now();

    let (store, owner_node_id, engine) = resolve_upload_context(&state).await?;

    let outcome = match store
        .blobs()
        .add_path(path)
        .await
    {
        Ok(tag_info) => {
            let h = tag_info.hash;
            let _ = store.tags().set(clean_name.as_bytes(), tag_info).await;
            h
        }
        Err(e) => {
            let err_msg = format!("failed streaming file import: {e}");
            emit_import_progress(
                Some(&app),
                &ImportProgressPayload {
                    import_id: import_id.clone(),
                    archive_name: clean_name.clone(),
                    current_entry: 0,
                    total_entries: 1,
                    bytes_processed: 0,
                    total_bytes: size_bytes,
                    current_file: clean_name.clone(),
                    bytes_per_sec: 0,
                    status: "error".to_string(),
                    error: Some(err_msg.clone()),
                },
            );
            return Err(err_msg);
        }
    };

    let hash = outcome.to_hex().to_string();

    // Auto-create parent folders
    if let Some((parent, _)) = clean_name.rsplit_once('/') {
        let _ = crate::storage::blob_tracker::create_virtual_folder(&state.db, parent);
    }

    let _ = crate::storage::blob_tracker::remove_deletion_tombstone(&state.db, &hash);

    upsert_blob(
        &state.db,
        &hash,
        Some(&clean_name),
        size_bytes,
        false,
        Some(&owner_node_id),
    )
    .map_err(|e| e.to_string())?;

    let _ = record_peer_seed(&state.db, &hash, &owner_node_id);

    // Record upload bandwidth
    state.bandwidth.record_upload(size_bytes);

    let record = BlobRecord {
        hash: hash.clone(),
        name: Some(clean_name.clone()),
        size_bytes,
        last_accessed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        is_pinned: false,
        owner_node_id: Some(owner_node_id.clone()),
        delete_votes: Vec::new(),
        keep_votes: Vec::new(),
        deletion_scheduled_at: None,
        seed_count: 1,
        is_cached: true,
        ..Default::default()
    };

    broadcast_catalog_event_opt(&engine, SyncMessage::FileAdded(record)).await;

    let elapsed = start_time.elapsed().as_secs_f64();
    let speed = if elapsed > 0.02 {
        (size_bytes as f64 / elapsed) as u64
    } else {
        0
    };

    emit_import_progress(
        Some(&app),
        &ImportProgressPayload {
            import_id,
            archive_name: clean_name.clone(),
            current_entry: 1,
            total_entries: 1,
            bytes_processed: size_bytes,
            total_bytes: size_bytes,
            current_file: clean_name.clone(),
            bytes_per_sec: speed,
            status: "completed".to_string(),
            error: None,
        },
    );

    Ok(hash)
}

/// Batch delete files owned by user or cast delete votes for community files.
#[tauri::command]
pub async fn batch_delete_files(
    hashes: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let mut deleted = Vec::new();
    let (my_id, engine, secret_key) = resolve_identity_context(&state).await?;
    let store = resolve_local_store(&state).await;

    for hash in hashes {
        let is_owner = match get_blob(&state.db, &hash) {
            Ok(Some(r)) => r
                .owner_node_id
                .as_deref()
                .map(|o| crate::storage::db::is_matching_node_id(o, &my_id))
                .unwrap_or(false),
            _ => false,
        };

        if is_owner {
            let now = crate::storage::blob_tracker::now_unix();
            let tombstone = crate::storage::blob_tracker::DeletionTombstone::create_signed(
                hash.clone(),
                my_id.clone(),
                now,
                &secret_key,
            );
            if crate::storage::blob_tracker::record_deletion_tombstone(&state.db, &tombstone).is_ok() {
                if let Some(store) = store.as_ref() {
                    let _ = store.tags().delete(hash.as_bytes()).await;
                }
                broadcast_catalog_event_opt(&engine, SyncMessage::FileDeletedTombstone(tombstone)).await;
                deleted.push(hash);
            }
        } else {
            // Community voting delete
            if cast_delete_vote(&state.db, &hash, &my_id).is_ok() {
                broadcast_catalog_event_opt(
                    &engine,
                    SyncMessage::FileVoteDelete {
                        hash: hash.clone(),
                        voter_node_id: my_id.clone(),
                    },
                )
                .await;
                deleted.push(hash);
            }
        }
    }
    Ok(deleted)
}

/// Batch move files to a specified target folder.
#[tauri::command]
pub async fn batch_move_files(
    hashes: Vec<String>,
    target_folder: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let mut moved = Vec::new();
    let clean_target = target_folder.trim().trim_matches('/');

    // Auto-create target folder if non-empty
    if !clean_target.is_empty() {
        let _ = crate::storage::blob_tracker::create_virtual_folder(&state.db, clean_target);
    }

    let (my_node_id, engine, _) = resolve_identity_context(&state).await?;

    for hash in hashes {
        if let Ok(Some(blob)) = get_blob(&state.db, &hash) {
            // SECURITY: Only allow owner to move files
            let is_owner = blob
                .owner_node_id
                .as_deref()
                .map(|o| crate::storage::db::is_matching_node_id(o, &my_node_id))
                .unwrap_or(true);

            if !is_owner {
                continue;
            }

            let old_name = blob.name.unwrap_or_else(|| hash.clone());
            let basename = old_name.split('/').next_back().unwrap_or(&old_name);
            let new_path = if clean_target.is_empty() {
                basename.to_string()
            } else {
                format!("{clean_target}/{basename}")
            };

            if let Ok(sanitized) = crate::storage::path_utils::sanitize_virtual_path(&new_path) {
                if rename_blob(&state.db, &hash, &sanitized).is_ok() {
                    broadcast_catalog_event_opt(
                        &engine,
                        SyncMessage::FileMoved {
                            hash: hash.clone(),
                            new_name: sanitized,
                        },
                    )
                    .await;
                    moved.push(hash);
                }
            }
        }
    }
    Ok(moved)
}

/// Cast a community rating on a file (-1, 0, or 1).
#[tauri::command]
pub async fn rate_file(
    hash: String,
    rating: i8,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not started")?;
    let my_id = engine.node_id().to_string();

    crate::storage::blob_tracker::rate_file(&state.db, &hash, &my_id, rating)
        .map_err(|e| e.to_string())?;

    broadcast_catalog_event(
        &engine.endpoint,
        &engine.sync_alpn,
        SyncMessage::FileRated {
            hash,
            voter_node_id: my_id,
            rating,
        },
    )
    .await;
    Ok(())
}

/// Toggle bookmark / star state for a file locally.
#[tauri::command]
pub async fn toggle_star(
    hash: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    crate::storage::blob_tracker::toggle_star(&state.db, &hash).map_err(|e| e.to_string())
}

/// Report a file as dangerous. Triggers emergency quarantine if 5 reports arrive in 1 hour.
#[tauri::command]
pub async fn report_dangerous(
    hash: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::storage::blob_tracker::QuarantineStatus, String> {
    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not started")?;
    let my_id = engine.node_id().to_string();

    let status = crate::storage::blob_tracker::report_dangerous(
        &state.db,
        &hash,
        &my_id,
        reason.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    broadcast_catalog_event(
        &engine.endpoint,
        &engine.sync_alpn,
        SyncMessage::FileQuarantineReport {
            hash,
            reporter_node_id: my_id,
            reason,
        },
    )
    .await;
    Ok(status)
}

/// Retrieve full quarantine status and reported reasons for a file.
#[tauri::command]
pub async fn get_quarantine_info(
    hash: String,
    state: State<'_, AppState>,
) -> Result<crate::storage::blob_tracker::QuarantineInfo, String> {
    crate::storage::blob_tracker::get_quarantine_info(&state.db, &hash).map_err(|e| e.to_string())
}

/// Create and persist a virtual folder, broadcasting it to the swarm.
#[tauri::command]
pub async fn create_folder(
    folder_path: Option<String>,
    virtual_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = folder_path.or(virtual_path).ok_or("Missing folder path parameter")?;
    crate::storage::blob_tracker::create_virtual_folder(&state.db, &path)
        .map_err(|e| e.to_string())?;

    if let Some(engine) = state.engine.read().await.as_ref() {
        broadcast_catalog_event(
            &engine.endpoint,
            &engine.sync_alpn,
            SyncMessage::FolderCreated { path: path.clone() },
        )
        .await;
    }
    Ok(())
}

/// List all explicitly persisted virtual folders.
#[tauri::command]
pub async fn list_folders(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    crate::storage::blob_tracker::list_virtual_folders(&state.db)
        .map_err(|e| e.to_string())
}

/// Delete a virtual folder. If `recursive` is true, recursively removes all contained files,
/// generates signed deletion tombstones for owned files, and cleans up storage.
#[tauri::command]
pub async fn delete_folder(
    folder_path: Option<String>,
    virtual_path: Option<String>,
    recursive: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = folder_path.or(virtual_path).ok_or("Missing folder path parameter")?;
    let is_recursive = recursive.unwrap_or(true);

    if is_recursive {
        // Destructive local actions must work even while the engine is still starting.
        let (our_node_id, engine_opt, secret_key) = resolve_identity_context(&state).await?;

        let (deleted_hashes, tombstones) = crate::storage::blob_tracker::delete_virtual_folder_recursive(
            &state.db,
            &path,
            Some(&our_node_id),
            Some(&secret_key),
        )
        .map_err(|e| e.to_string())?;

        // Prune raw blob storage tags for deleted hashes
        if let Some(engine) = engine_opt.as_ref() {
            for h in &deleted_hashes {
                let _ = engine.store.tags().delete(h.as_bytes()).await;
            }

            // Broadcast deletion tombstones first
            if !tombstones.is_empty() {
                broadcast_catalog_event(
                    &engine.endpoint,
                    &engine.sync_alpn,
                    SyncMessage::TombstoneDump(tombstones),
                )
                .await;
            }

            // Broadcast FolderDeleted
            broadcast_catalog_event(
                &engine.endpoint,
                &engine.sync_alpn,
                SyncMessage::FolderDeleted { path: path.clone() },
            )
            .await;
        }

        // Clean up orphaned disk blobs to keep disk and database in lockstep
        let blob_dir = state.db.resolve_blob_dir();
        let _ = crate::storage::path_utils::cleanup_orphaned_disk_blobs(&blob_dir, &state.db);
    } else {
        crate::storage::blob_tracker::delete_virtual_folder(&state.db, &path)
            .map_err(|e| e.to_string())?;

        if let Some(engine) = state.engine.read().await.as_ref() {
            broadcast_catalog_event(
                &engine.endpoint,
                &engine.sync_alpn,
                SyncMessage::FolderDeleted { path: path.clone() },
            )
            .await;
        }
    }

    Ok(())
}

/// Return the active transfer resume queue.
#[tauri::command]
pub async fn get_transfer_queue(
    state: State<'_, AppState>,
) -> Result<Vec<crate::storage::blob_tracker::TransferQueueItem>, String> {
    crate::storage::blob_tracker::list_resume_queue(&state.db).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_dangerous_file_extension_blocking() {
        // Block dangerous execution / script payloads
        assert!(is_dangerous_file_extension(Path::new("malware.exe")));
        assert!(is_dangerous_file_extension(Path::new("script.bat")));
        assert!(is_dangerous_file_extension(Path::new("payload.cmd")));
        assert!(is_dangerous_file_extension(Path::new("macro.vbs")));
        assert!(is_dangerous_file_extension(Path::new("autorun.ps1")));
        assert!(is_dangerous_file_extension(Path::new("installer.msi")));
        assert!(is_dangerous_file_extension(Path::new("screensaver.scr")));
        assert!(is_dangerous_file_extension(Path::new("app.hta")));
        assert!(is_dangerous_file_extension(Path::new("app.jar")));
        assert!(is_dangerous_file_extension(Path::new("shortcut.lnk")));
        assert!(is_dangerous_file_extension(Path::new("disk.iso")));
        assert!(is_dangerous_file_extension(Path::new("lib.dll")));

        // Allow benign documents and media
        assert!(!is_dangerous_file_extension(Path::new("lecture_notes.pdf")));
        assert!(!is_dangerous_file_extension(Path::new("readme.md")));
        assert!(!is_dangerous_file_extension(Path::new("assignment.txt")));
        assert!(!is_dangerous_file_extension(Path::new("diagram.png")));
        assert!(!is_dangerous_file_extension(Path::new("photo.jpg")));
        assert!(!is_dangerous_file_extension(Path::new("archive.zip")));
        assert!(!is_dangerous_file_extension(Path::new("dataset.csv")));
    }

    #[test]
    fn test_executable_masquerading_detection() {
        // Windows MZ executable header disguised as pdf/png/txt
        let mz_header = b"MZ\x90\x00\x03\x00\x00\x00\x04\x00";
        assert!(is_executable_masquerading_as_doc(Path::new("syllabus.pdf"), mz_header));
        assert!(is_executable_masquerading_as_doc(Path::new("chart.png"), mz_header));
        assert!(is_executable_masquerading_as_doc(Path::new("notes.txt"), mz_header));

        // Linux ELF binary disguised as pdf
        let elf_header = b"\x7fELF\x02\x01\x01\x00";
        assert!(is_executable_masquerading_as_doc(Path::new("homework.pdf"), elf_header));

        // Legitimate PDF (starts with %PDF-)
        let real_pdf = b"%PDF-1.7\n%\xb5\xed\xae\xfb\n";
        assert!(!is_executable_masquerading_as_doc(Path::new("homework.pdf"), real_pdf));

        // Legitimate PNG (starts with \x89PNG)
        let real_png = b"\x89PNG\r\n\x1a\n";
        assert!(!is_executable_masquerading_as_doc(Path::new("diagram.png"), real_png));

        // Legitimate plain text
        let real_text = b"Hello world! This is my lecture notes.";
        assert!(!is_executable_masquerading_as_doc(Path::new("notes.txt"), real_text));
    }
}
