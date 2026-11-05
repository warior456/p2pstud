// src-tauri/src/lib.rs
// Tauri app setup: initialise shared state, register IPC commands, system tray, and background workers.

pub mod commands;
pub mod p2p;
pub mod storage;

use std::sync::Arc;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tokio::sync::RwLock;

use crate::p2p::bandwidth::BandwidthTracker;
use crate::p2p::engine::IrohEngine;
use crate::storage::db::Database;

/// Shared application state injected into every Tauri command via `tauri::State`.
pub struct AppState {
    pub engine: Arc<RwLock<Option<IrohEngine>>>,
    pub db: Arc<Database>,
    pub bandwidth: Arc<BandwidthTracker>,
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
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
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
            let bandwidth = Arc::new(BandwidthTracker::new());

            let state = AppState {
                engine: engine.clone(),
                db: db.clone(),
                bandwidth: bandwidth.clone(),
            };
            app.manage(state);

            // Create system tray icon and background persistence menu
            let show_item = MenuItem::with_id(app, "show", "Open P2P Campus Workspace", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit P2P Workspace", true, None::<&str>)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            if let Some(icon) = app.default_window_icon() {
                let _ = TrayIconBuilder::with_id("main-tray")
                    .icon(icon.clone())
                    .menu(&tray_menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app_handle, event| match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app_handle = tray.app_handle();
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let is_visible = window.is_visible().unwrap_or(false);
                                if is_visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    })
                    .build(app);
            }

            // Spawn the Iroh node startup on the async runtime.
            let data_dir_clone = data_dir.clone();
            let db_clone = db.clone();
            let engine_clone = engine.clone();
            let bw_clone = bandwidth.clone();
            let app_handle_for_workers = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match p2p::engine::IrohEngine::start(&data_dir_clone, db_clone.clone()).await {
                    Ok(e) => {
                        tracing::info!("Iroh node started: {}", e.node_id());
                        *engine_clone.write().await = Some(e);
                        // Start background eviction, hosting, scarcity, and pruning workers
                        storage::eviction::spawn_eviction_worker(db_clone.clone(), engine_clone.clone());
                        storage::hosting_worker::spawn_hosting_worker(db_clone.clone(), engine_clone.clone(), bw_clone.clone());
                        storage::scarcity::spawn_scarcity_worker(db_clone.clone(), engine_clone.clone(), bw_clone, app_handle_for_workers.clone());
                        storage::pruning::spawn_lifecycle_pruning_worker(db_clone.clone(), engine_clone.clone());
                        p2p::network_profile::spawn_network_profile_monitor(db_clone, app_handle_for_workers);
                    }
                    Err(err) => {
                        tracing::error!("Failed to start Iroh node: {:#}", err);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Background persistence: clicking [X] hides the window to the system tray
                // so that background P2P file seeding continues uninterrupted.
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::storage_cmds::get_storage_stats,
            commands::storage_cmds::get_app_settings,
            commands::storage_cmds::update_app_settings,
            commands::storage_cmds::set_network_secret,
            commands::storage_cmds::set_cache_limit,
            commands::storage_cmds::set_full_mirror,
            commands::storage_cmds::purge_cache,
            commands::storage_cmds::purge_unpinned_cache,
            commands::storage_cmds::open_data_dir,
            commands::storage_cmds::get_setting,
            commands::storage_cmds::set_setting,
            commands::file_cmds::pin_file,
            commands::file_cmds::unpin_file,
            commands::file_cmds::list_files,
            commands::file_cmds::read_file_bytes,
            commands::file_cmds::add_file,
            commands::file_cmds::upload_file_bytes,
            commands::file_cmds::import_zip_bytes,
            commands::file_cmds::import_zip_file,
            commands::file_cmds::delete_file,
            commands::file_cmds::vote_delete_file,
            commands::file_cmds::unvote_delete_file,
            commands::file_cmds::vote_keep_file,
            commands::file_cmds::unvote_keep_file,
            commands::file_cmds::evict_local_file,
            commands::file_cmds::export_file,
            commands::file_cmds::move_file,
            commands::file_cmds::rate_file,
            commands::file_cmds::toggle_star,
            commands::file_cmds::report_dangerous,
            commands::file_cmds::create_folder,
            commands::file_cmds::list_folders,
            commands::file_cmds::delete_folder,
            commands::file_cmds::get_transfer_queue,
            commands::peer_cmds::list_peers,
            commands::peer_cmds::get_node_id,
            commands::peer_cmds::get_node_ticket,
            commands::peer_cmds::get_node_addr,
            commands::peer_cmds::connect_peer,
            commands::peer_cmds::get_network_profile,
            commands::peer_cmds::set_network_profile_override,
            commands::peer_cmds::add_ssid_rule,
            commands::peer_cmds::list_ssid_rules,
            commands::peer_cmds::delete_ssid_rule,
            commands::peer_cmds::get_sole_seed_files,
            commands::peer_cmds::reconnect_swarm,
            commands::diagnostic_cmds::run_connection_doctor,
            commands::backup_cmds::export_app_backup,
            commands::backup_cmds::export_folder_zip_preflight,
            commands::backup_cmds::export_folder_zip,
            commands::backup_cmds::check_app_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
