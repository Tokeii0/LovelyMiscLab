//! Application state managed by Tauri and injected into commands via
//! `tauri::State<AppState>`.

use std::sync::{Arc, Mutex};

use misclab_core::graph::executor::NodeCache;
use misclab_core::node::registry::NodeRegistry;
use misclab_core::node::NodeEnv;

use crate::db::Db;
use crate::jobs::JobManager;

pub struct AppState {
    pub db: Db,
    /// The node registry, built once at startup (drives the palette + execution).
    pub registry: Arc<NodeRegistry>,
    /// Tracks running graph jobs for cancellation.
    pub jobs: JobManager,
    /// Incremental-execution cache, shared across runs (live mode).
    pub cache: Arc<Mutex<NodeCache>>,
    /// App settings (AI config + default output dir), persisted to settings.json.
    pub settings: Arc<Mutex<NodeEnv>>,
}
