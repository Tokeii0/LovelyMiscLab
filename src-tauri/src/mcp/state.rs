//! Shared state for the MCP server and the whole-canvas snapshot exchanged with
//! the frontend.
//!
//! [`McpState`] is a cheap, `Clone`-able projection of the app's [`AppState`]
//! that holds only what the tools need — deliberately **excluding** `db`
//! (rusqlite is `!Sync`) and `jobs`. It's safe to move into the server thread and
//! to clone per rmcp session.

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use misclab_core::graph::composite::CompositeModule;
use misclab_core::graph::executor::NodeCache;
use misclab_core::graph::model::{Edge, NodeInstance, PortRef, SerializedGraph};
use misclab_core::graph::script_node::ScriptModule;
use misclab_core::node::registry::NodeRegistry;
use misclab_core::node::NodeEnv;

// ---------------------------------------------------------------------------
// Canvas snapshot — mirrors the frontend `SavedNode`/`SavedEdge`/`FlowProject`
// (a superset of the executable graph, so round-trips are lossless).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pos {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasNode {
    pub id: String,
    pub descriptor_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub input_params: Vec<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub position: Pos,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasEdge {
    pub id: String,
    pub source: String,
    #[serde(default)]
    pub source_handle: Option<String>,
    pub target: String,
    #[serde(default)]
    pub target_handle: Option<String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub edge_type: Option<String>,
}

/// The whole-canvas snapshot. `rev` is a monotonic revision used to break the
/// frontend↔backend echo loop (see the canvas bridge).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasSnapshot {
    #[serde(default)]
    pub nodes: Vec<CanvasNode>,
    #[serde(default)]
    pub edges: Vec<CanvasEdge>,
    #[serde(default)]
    pub rev: u64,
}

impl CanvasSnapshot {
    /// Lower this rich snapshot to an executable [`SerializedGraph`], mirroring
    /// the frontend `buildGraph()`: drop disabled nodes and any edge that touches
    /// a disabled node or is missing its port handles.
    pub fn to_serialized_graph(&self) -> SerializedGraph {
        let enabled: std::collections::HashSet<&str> = self
            .nodes
            .iter()
            .filter(|n| !n.disabled)
            .map(|n| n.id.as_str())
            .collect();

        let nodes = self
            .nodes
            .iter()
            .filter(|n| !n.disabled)
            .map(|n| NodeInstance {
                id: n.id.clone(),
                descriptor_id: n.descriptor_id.clone(),
                params: if n.params.is_null() {
                    serde_json::json!({})
                } else {
                    n.params.clone()
                },
                position: (n.position.x as f32, n.position.y as f32),
            })
            .collect();

        let edges = self
            .edges
            .iter()
            .filter_map(|e| {
                let sh = e.source_handle.as_deref()?;
                let th = e.target_handle.as_deref()?;
                if !enabled.contains(e.source.as_str()) || !enabled.contains(e.target.as_str()) {
                    return None;
                }
                Some(Edge {
                    from: PortRef {
                        node: e.source.clone(),
                        port: sh.to_string(),
                    },
                    to: PortRef {
                        node: e.target.clone(),
                        port: th.to_string(),
                    },
                })
            })
            .collect();

        SerializedGraph { nodes, edges }
    }
}

// ---------------------------------------------------------------------------
// Persisted MCP server settings (app-shell config, kept out of core NodeEnv).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct McpSettings {
    /// Auto-start the server when the app launches.
    pub enabled: bool,
    /// TCP port for the local endpoint.
    pub port: u16,
    /// Bearer token clients must present. Generated on first enable.
    pub token: Option<String>,
    /// Bind to 0.0.0.0 instead of 127.0.0.1 (advanced; discouraged).
    pub bind_all: bool,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8765,
            token: None,
            bind_all: false,
        }
    }
}

const CONFIG_FILE: &str = "mcp.json";

pub fn load_config(dir: &Path) -> McpSettings {
    std::fs::read_to_string(dir.join(CONFIG_FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(dir: &Path, cfg: &McpSettings) -> std::io::Result<()> {
    std::fs::create_dir_all(dir).ok();
    let json = serde_json::to_string_pretty(cfg).unwrap_or_else(|_| "{}".into());
    std::fs::write(dir.join(CONFIG_FILE), json)
}

// ---------------------------------------------------------------------------
// McpState — the Clone-able projection handed to the server.
// ---------------------------------------------------------------------------

/// The bits of the host app the tools actually need — emitting canvas updates
/// and resolving the data dir. Abstracted behind a trait so `McpState` can be
/// built (and the server exercised) in tests without a Tauri `AppHandle`.
pub trait AppBridge: Send + Sync {
    fn emit_canvas(&self, snapshot: &CanvasSnapshot);
    fn app_data_dir(&self) -> Option<std::path::PathBuf>;
}

/// Production bridge over a Tauri `AppHandle`.
pub struct TauriBridge(pub tauri::AppHandle);

impl AppBridge for TauriBridge {
    fn emit_canvas(&self, snapshot: &CanvasSnapshot) {
        use tauri::Emitter;
        let _ = self.0.emit("mcp://canvas-update", snapshot);
    }
    fn app_data_dir(&self) -> Option<std::path::PathBuf> {
        use tauri::Manager;
        self.0.path().app_data_dir().ok()
    }
}

#[derive(Clone)]
pub struct McpState {
    pub registry: Arc<NodeRegistry>,
    pub composites: Arc<Mutex<Vec<CompositeModule>>>,
    pub scripts: Arc<Mutex<Vec<ScriptModule>>>,
    pub cache: Arc<Mutex<NodeCache>>,
    pub settings: Arc<Mutex<NodeEnv>>,
    pub canvas: Arc<Mutex<CanvasSnapshot>>,
    /// Emits `mcp://canvas-update` events and resolves the app data dir.
    pub app: Arc<dyn AppBridge>,
    /// Bearer token (also used by the auth middleware).
    pub token: Arc<Option<String>>,
}

impl McpState {
    pub fn from_app(state: &crate::state::AppState, app: tauri::AppHandle, token: Option<String>) -> Self {
        Self {
            registry: state.registry.clone(),
            composites: state.composites.clone(),
            scripts: state.scripts.clone(),
            cache: state.cache.clone(),
            settings: state.settings.clone(),
            canvas: state.canvas.clone(),
            app: Arc::new(TauriBridge(app)),
            token: Arc::new(token),
        }
    }

    /// Built-ins + composites + script nodes (mirrors `commands::graph`).
    pub fn combined_registry(&self) -> NodeRegistry {
        let comps = self.composites.lock().expect("composites mutex poisoned");
        let scripts = self.scripts.lock().expect("scripts mutex poisoned");
        crate::state::combined_registry_from(self.registry.as_ref(), &comps, &scripts)
    }
}
