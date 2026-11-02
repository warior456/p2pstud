// src-tauri/src/commands/file_cmds.rs
// Tauri IPC commands for file operations: pin/unpin, read, add.

use tauri::{AppHandle, Emitter, State};

use crate::storage::blob_tracker::{
    list_all_blobs, set_pinned, touch_blob, upsert_blob, BlobRecord,
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

/// Read a file's bytes from the local iroh-blobs store.
///
/// For files ≤ 8 MB: returns all bytes directly.
/// For files > 8 MB: streams chunks as Tauri events `file-chunk` and returns Ok(vec![]).
/// The frontend must listen for `file-chunk` events when expecting large files.
///
/// Event payload: `{ hash: string, offset: number, data: number[] }`
#[tauri::command]
pub async fn read_file_bytes(
    hash: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<u8>, String> {
    const INLINE_LIMIT: u64 = 8 * 1024 * 1024; // 8 MB

    // Update access timestamp (LRU tracking).
    touch_blob(&state.db, &hash).map_err(|e| e.to_string())?;

    let parsed_hash = hash
        .parse::<iroh_blobs::Hash>()
        .map_err(|e| format!("invalid hash '{hash}': {e}"))?;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    // Fetch bytes from the local blob store.
    let data = engine
        .blobs
        .clone()
        .client()
        .read_to_bytes(parsed_hash)
        .await
        .map_err(|e| format!("blob read failed: {e}"))?;

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

    Ok(vec![]) // Frontend reconstructs from events.
}

/// Add a local file to the workspace blob store.
///
/// Reads `local_path` from disk, imports it into iroh-blobs (computing the BLAKE3 hash),
/// records it in SQLite, and returns the BLAKE3 hex hash.
#[tauri::command]
pub async fn add_file(local_path: String, state: State<'_, AppState>) -> Result<String, String> {
    let path = std::path::Path::new(&local_path);

    // Security: verify the file actually exists and is a file (not a symlink to /dev/mem etc.).
    let meta = std::fs::metadata(path).map_err(|e| format!("cannot stat '{}': {e}", local_path))?;

    if !meta.is_file() {
        return Err(format!("'{}' is not a regular file", local_path));
    }

    let data = std::fs::read(path).map_err(|e| format!("read failed: {e}"))?;
    let size_bytes = data.len() as u64;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    // Import into iroh-blobs; this computes the BLAKE3 hash and writes chunks.
    let outcome = engine
        .blobs
        .clone()
        .client()
        .add_bytes(bytes::Bytes::from(data))
        .await
        .map_err(|e| format!("blob import failed: {e}"))?;

    let hash = outcome.hash.to_hex().to_string();

    // Record in SQLite.
    upsert_blob(&state.db, &hash, size_bytes, false).map_err(|e| e.to_string())?;

    tracing::info!(hash = %hash, bytes = size_bytes, "Added file to workspace");
    Ok(hash)
}
