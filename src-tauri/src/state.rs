//! Application state managed by Tauri and injected into commands via
//! `tauri::State<AppState>`.

use std::sync::{Arc, Mutex};

use misclab_core::graph::composite::CompositeModule;
use misclab_core::graph::executor::NodeCache;
use misclab_core::graph::script_node::ScriptModule;
use misclab_core::node::registry::NodeRegistry;
use misclab_core::node::NodeEnv;

use crate::db::Db;
use crate::jobs::JobManager;

pub struct AppState {
    pub db: Db,
    /// Built-in node registry, built once at startup. The effective registry for
    /// palette + execution is this plus the user's `composites` (merged on demand).
    pub registry: Arc<NodeRegistry>,
    /// User-defined composite (sub-graph) modules, loaded at startup and edited at
    /// runtime; persisted as JSON under `<app_data_dir>/modules/`.
    pub composites: Arc<Mutex<Vec<CompositeModule>>>,
    /// User-defined script/program nodes; persisted under `<app_data_dir>/script_modules/`.
    pub scripts: Arc<Mutex<Vec<ScriptModule>>>,
    /// Tracks running graph jobs for cancellation.
    pub jobs: JobManager,
    /// Incremental-execution cache, shared across runs (live mode).
    pub cache: Arc<Mutex<NodeCache>>,
    /// App settings (AI config + default output dir), persisted to settings.json.
    pub settings: Arc<Mutex<NodeEnv>>,
}
