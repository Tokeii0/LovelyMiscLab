//! LovelyMiscLab Tauri application shell — a thin adapter over `misclab-core`.
//! Registers plugins, builds the node registry, bootstraps the SQLite database
//! into managed state, and exposes the command surface.

mod commands;
mod db;
mod error;
mod jobs;
mod settings;
mod state;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(|app| {
            // App data dir holds the SQLite DB, artifact dirs, dictionaries, etc.
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            let db_path = data_dir.join("lovelymisclab.db");
            let db = db::Db::open(&db_path).expect("failed to open database");

            let registry = Arc::new(misclab_core::nodes::default_registry());
            let app_settings = settings::load(&data_dir);

            app.manage(AppState {
                db,
                registry,
                jobs: jobs::JobManager::default(),
                cache: Arc::new(Mutex::new(Default::default())),
                settings: Arc::new(Mutex::new(app_settings)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::ping,
            commands::system::app_info,
            commands::system::db_health,
            commands::graph::list_node_descriptors,
            commands::graph::run_node,
            commands::graph::run_graph,
            commands::graph::cancel_job,
            commands::graph::reset_run,
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::detect_tool,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
