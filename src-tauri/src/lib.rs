//! LovelyMiscLab Tauri application shell — a thin adapter over `misclab-core`.
//! Registers plugins, builds the node registry, loads settings and user modules
//! into managed state, and exposes the command surface.

mod commands;
mod error;
mod jobs;
#[cfg(feature = "mcp")]
mod mcp;
mod modules;
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
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(|app| {
            // Remove any leftover files from a previous self-update.
            commands::update::cleanup_leftovers();

            // App data dir holds settings, user modules and MCP config.
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            let registry = Arc::new(misclab_core::nodes::default_registry());
            let app_settings = settings::load(&data_dir);
            let mut composites: Vec<misclab_core::graph::composite::CompositeModule> =
                modules::load_all(&data_dir, "modules");
            composites.sort_by(|a, b| a.name.cmp(&b.name));
            let mut scripts: Vec<misclab_core::graph::script_node::ScriptModule> =
                modules::load_all(&data_dir, "script_modules");
            scripts.sort_by(|a, b| a.name.cmp(&b.name));

            app.manage(AppState {
                registry,
                composites: Arc::new(Mutex::new(composites)),
                scripts: Arc::new(Mutex::new(scripts)),
                jobs: jobs::JobManager::default(),
                cache: Arc::new(Mutex::new(Default::default())),
                settings: Arc::new(Mutex::new(app_settings)),
                #[cfg(feature = "mcp")]
                canvas: Arc::new(Mutex::new(Default::default())),
                #[cfg(feature = "mcp")]
                mcp: Arc::new(Mutex::new(None)),
            });

            // Auto-start the embedded MCP server if the user enabled it.
            #[cfg(feature = "mcp")]
            {
                let cfg = mcp::state::load_config(&data_dir);
                if cfg.enabled {
                    let st = app.state::<AppState>();
                    let mcp_state = mcp::McpState::from_app(
                        st.inner(),
                        app.handle().clone(),
                        cfg.token.clone(),
                    );
                    let host = if cfg.bind_all {
                        [0, 0, 0, 0]
                    } else {
                        [127, 0, 0, 1]
                    };
                    let addr = std::net::SocketAddr::from((host, cfg.port));
                    match mcp::start(mcp_state, addr) {
                        Ok(h) => *st.mcp.lock().expect("mcp mutex poisoned") = Some(h),
                        Err(e) => eprintln!("[mcp] auto-start failed: {e}"),
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::app_info,
            commands::graph::list_node_descriptors,
            commands::graph::run_node,
            commands::graph::run_node_streamed,
            commands::graph::run_graph,
            commands::graph::cancel_job,
            commands::graph::reset_run,
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::detect_tool,
            commands::ai_workflow::generate_workflow,
            commands::galgame::galgame_step,
            commands::modules::list_composite_modules,
            commands::modules::save_composite_module,
            commands::modules::delete_composite_module,
            commands::script_modules::list_script_modules,
            commands::script_modules::save_script_module,
            commands::script_modules::delete_script_module,
            commands::project::save_project,
            commands::project::load_project,
            commands::ai_workflow::explain_workflow,
            commands::ai_workflow::suggest_next_nodes,
            commands::agent::agent_run,
            commands::update::check_update,
            commands::update::install_update,
            #[cfg(feature = "mcp")]
            commands::mcp::mcp_start,
            #[cfg(feature = "mcp")]
            commands::mcp::mcp_stop,
            #[cfg(feature = "mcp")]
            commands::mcp::mcp_status,
            #[cfg(feature = "mcp")]
            commands::mcp::mcp_get_config,
            #[cfg(feature = "mcp")]
            commands::mcp::mcp_set_config,
            #[cfg(feature = "mcp")]
            commands::mcp::sync_canvas,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, _event| {
            // Gracefully stop the embedded MCP server on app exit.
            #[cfg(feature = "mcp")]
            if let tauri::RunEvent::Exit = _event {
                if let Some(state) = _app_handle.try_state::<AppState>() {
                    if let Some(handle) = state.mcp.lock().expect("mcp mutex poisoned").take() {
                        handle.stop();
                    }
                }
            }
        });
}
