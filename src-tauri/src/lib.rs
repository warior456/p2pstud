// src-tauri/src/lib.rs
// Tauri app setup: initialise shared state, register IPC commands, start background workers.

pub mod commands;
pub mod p2p;
pub mod storage;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

use crate::p2p::engine::IrohEngine;
use crate::storage::db::Database;

/// Shared application state injected into every Tauri command via `tauri::State`.
pub struct AppState {
    pub engine: Arc<RwLock<Option<IrohEngine>>>,
    pub db: Arc<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise logging (respects RUST_LOG env var).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "p2pstud=info,iroh=warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // dialog plugin not needed
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&data_dir)?;

            // Open SQLite database (runs migrations on first start).
            let db = Arc::new(
                storage::db::Database::open(&data_dir.join("p2pstud.db"))
                    .expect("failed to open database"),
            );

            // Store state; Iroh engine is started asynchronously after setup.
            let engine: Arc<RwLock<Option<IrohEngine>>> = Arc::new(RwLock::new(None));

            let state = AppState {
                engine: engine.clone(),
                db: db.clone(),
            };
            app.manage(state);

            // Spawn the Iroh node startup on the async runtime.
            let data_dir_clone = data_dir.clone();
            tauri::async_runtime::spawn(async move {
                match p2p::engine::IrohEngine::start(&data_dir_clone, db).await {
                    Ok(e) => {
                        tracing::info!("Iroh node started: {}", e.node_id());
                        *engine.write().await = Some(e);
                    }
                    Err(err) => {
                        tracing::error!("Failed to start Iroh node: {:#}", err);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::storage_cmds::get_storage_stats,
            commands::storage_cmds::set_cache_limit,
            commands::file_cmds::pin_file,
            commands::file_cmds::unpin_file,
            commands::file_cmds::read_file_bytes,
            commands::file_cmds::add_file,
            commands::peer_cmds::list_peers,
            commands::peer_cmds::get_node_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

