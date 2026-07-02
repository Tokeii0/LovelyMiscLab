//! The rmcp server definition: the tool surface (`#[tool]` methods) plus the
//! axum wiring that serves it over streamable-HTTP behind the bearer gate.
//!
//! The `#[tool]` methods are thin wrappers — the real logic lives in
//! [`crate::mcp::handlers`] (transport-agnostic, unit-testable).

use std::net::SocketAddr;

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::mcp::state::McpState;

const INSTRUCTIONS: &str = "LovelyMiscLab —— 基于节点图的 CTF misc 工具箱（约 116 个内置节点 + 用户自定义模块）。\n\
发现节点请循序渐进以节省 token：先 `list_categories` 看分类，再 `list_nodes`(带 category) 列出该类节点（默认只返回 id+名称），\
真正要用某节点时再 `describe_node` 查它的输入/输出/参数。不要无参数调用 `list_nodes` 拉全量。\n\
执行：`run_node` 跑单个节点，`run_graph` 跑整条工作流（不带 graph 即运行用户当前画布）。\n\
画布协作：`get_canvas` 读取用户画布，`add_node`/`connect`/`set_param` 等增量修改，或 `set_canvas` 整体替换。\n\
端口值形如 {\"type\":\"text\",\"value\":\"...\"}；二进制建议用文件路径传递。";

// ---- tool argument schemas --------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListNodesArgs {
    /// Filter to an exact category (as returned by `list_categories`).
    #[serde(default)]
    pub category: Option<String>,
    /// Case-insensitive substring over node id / display name.
    #[serde(default)]
    pub query: Option<String>,
    /// Include inputs/outputs/params (heavier). Default is id+name+category only.
    #[serde(default)]
    pub detail: Option<bool>,
    /// Return full NodeDescriptors (heaviest). Prefer `describe_node` for one node.
    #[serde(default)]
    pub full: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DescribeNodeArgs {
    /// The node type id, e.g. "base64_decode".
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DetectToolArgs {
    /// Executable path or name, e.g. "python".
    pub path: String,
    /// Version flag to run; defaults to "--version".
    #[serde(default)]
    pub arg: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunNodeArgs {
    /// Node type id, e.g. "base64_decode" (see `list_nodes`).
    pub descriptor_id: String,
    /// Port name → value. Values: a bare string/number/bool, or {type,value}.
    /// Bytes: {"type":"bytes","value":{"base64":"..."}} or {"path":"..."}.
    #[serde(default)]
    pub inputs: serde_json::Value,
    /// The node's parameter object (see the node's `params`).
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunGraphArgs {
    /// Optional graph to run: {"nodes":[{"id","descriptorId","params"}],
    /// "edges":[{"from":{"node","port"},"to":{"node","port"}}]}. Omit to run the
    /// user's current canvas.
    #[serde(default)]
    pub graph: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetCanvasArgs {
    /// Whole-canvas snapshot: {"nodes":[SavedNode...],"edges":[SavedEdge...]}.
    pub snapshot: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddNodeArgs {
    /// Node type id (see `list_nodes`).
    pub descriptor_id: String,
    /// Optional param overrides (defaults come from the descriptor).
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    /// Optional canvas x/y.
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectArgs {
    /// Source node id.
    pub source: String,
    /// Source output port name.
    pub source_handle: String,
    /// Target node id.
    pub target: String,
    /// Target input port (or promoted param) name.
    pub target_handle: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetParamArgs {
    pub node_id: String,
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeIdArgs {
    pub node_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EdgeIdArgs {
    pub edge_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MoveNodeArgs {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveWorkflowArgs {
    /// Destination path ending in `.lml` or `.json`.
    pub path: String,
    /// Optional snapshot to save; omit to save the current canvas.
    #[serde(default)]
    pub snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoadWorkflowArgs {
    /// Path to a `.lml`/`.json` flow file.
    pub path: String,
    /// If true, also load it onto the user's canvas.
    #[serde(default)]
    pub apply: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModuleArgs {
    /// The module JSON (CompositeModule or ScriptModule shape).
    pub module: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerateWorkflowArgs {
    /// Natural-language task description.
    pub prompt: String,
    /// If true, also apply the generated graph to the user's canvas.
    #[serde(default)]
    pub apply: Option<bool>,
}

// ---- result helpers ---------------------------------------------------------

/// Wrap a JSON value as a tool result (serialized to a text content block, which
/// every MCP client renders).
fn json_ok(value: serde_json::Value) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(&value)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// Map a handler `Result<Value, String>` into a tool result (errors → invalid_params).
fn json_res(r: Result<serde_json::Value, String>) -> Result<CallToolResult, McpError> {
    match r {
        Ok(v) => json_ok(v),
        Err(e) => Err(McpError::invalid_params(e, None)),
    }
}

/// The MCP server instance. rmcp builds a fresh one per session via the factory
/// closure in [`serve`], each sharing the same [`McpState`] (cheap `Clone`).
#[derive(Clone)]
pub struct McpServer {
    state: McpState,
    // Read by the `#[tool_handler]`-generated code, which the dead-code lint misses.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl McpServer {
    pub fn new(state: McpState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    /// Liveness probe — returns `"pong"`.
    #[tool(description = "Health check for the LovelyMiscLab MCP server. Returns 'pong'.")]
    async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text("pong")]))
    }

    /// Node categories with counts — the cheapest discovery step.
    #[tool(
        description = "List node categories with counts. Call this FIRST, then list_nodes with a category — avoids dumping all ~116 nodes."
    )]
    async fn list_categories(&self) -> Result<CallToolResult, McpError> {
        json_ok(self.state.list_categories())
    }

    /// Discover available node types.
    #[tool(
        description = "List node types. Filter by `category` (from list_categories) or `query` (substring). Returns compact id+name by default; pass `detail`=true for ports/params, or use describe_node for one node. Token-efficient: filter by category rather than listing all."
    )]
    async fn list_nodes(
        &self,
        Parameters(args): Parameters<ListNodesArgs>,
    ) -> Result<CallToolResult, McpError> {
        json_ok(self.state.list_nodes(
            args.category,
            args.query,
            args.detail.unwrap_or(false),
            args.full.unwrap_or(false),
        ))
    }

    /// Full schema for one node type.
    #[tool(description = "Get the full descriptor (inputs, outputs, params) for one node type id.")]
    async fn describe_node(
        &self,
        Parameters(args): Parameters<DescribeNodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.state.describe_node(&args.id) {
            Ok(v) => json_ok(v),
            Err(e) => Err(McpError::invalid_params(e, None)),
        }
    }

    /// The user's saved composite + script modules.
    #[tool(description = "List the user's saved composite (sub-graph) and script modules.")]
    async fn list_modules(&self) -> Result<CallToolResult, McpError> {
        json_ok(self.state.list_modules())
    }

    /// App settings, with AI API keys redacted.
    #[tool(
        description = "Read app settings (AI model config, output dir, external tool paths). API keys are redacted."
    )]
    async fn get_settings(&self) -> Result<CallToolResult, McpError> {
        json_ok(self.state.get_settings_redacted())
    }

    /// Probe an external tool.
    #[tool(description = "Check whether an external tool (e.g. python, 7z) is available and its version.")]
    async fn detect_tool(
        &self,
        Parameters(args): Parameters<DetectToolArgs>,
    ) -> Result<CallToolResult, McpError> {
        let DetectToolArgs { path, arg } = args;
        let status = tokio::task::spawn_blocking(move || {
            crate::commands::settings::detect_tool_impl(&path, arg)
        })
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        json_ok(serde_json::to_value(status).unwrap_or_default())
    }

    /// Run a single node standalone.
    #[tool(
        description = "Run one node with the given inputs and params; returns its output ports. Use for quick one-off operations (decode, hash, ...)."
    )]
    async fn run_node(
        &self,
        Parameters(args): Parameters<RunNodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let out = tokio::task::spawn_blocking(move || {
            state.run_node(&args.descriptor_id, &args.inputs, &args.params)
        })
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .map_err(|e| McpError::internal_error(e, None))?;
        json_ok(out)
    }

    /// Run a whole graph / the current canvas.
    #[tool(
        description = "Run a full node graph (a pipeline) and return every node's outputs. Omit `graph` to run the user's current canvas. Chains multiple operations."
    )]
    async fn run_graph(
        &self,
        Parameters(args): Parameters<RunGraphArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let out = tokio::task::spawn_blocking(move || state.run_graph(args.graph))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .map_err(|e| McpError::internal_error(e, None))?;
        json_ok(out)
    }

    /// Read the user's current on-screen canvas.
    #[tool(
        description = "Read the user's current canvas (nodes + edges) exactly as shown on screen. Call this before modifying the canvas."
    )]
    async fn get_canvas(&self) -> Result<CallToolResult, McpError> {
        json_ok(self.state.get_canvas())
    }

    /// Replace the whole canvas.
    #[tool(
        description = "Replace the user's entire canvas with a new snapshot {nodes,edges}. Prefer the granular add_node/connect tools for incremental edits."
    )]
    async fn set_canvas(
        &self,
        Parameters(a): Parameters<SetCanvasArgs>,
    ) -> Result<CallToolResult, McpError> {
        json_res(self.state.set_canvas(a.snapshot))
    }

    /// Add a node to the canvas.
    #[tool(
        description = "Add a node to the user's canvas; returns the new node id. Params default from the descriptor and can be overridden."
    )]
    async fn add_node(
        &self,
        Parameters(a): Parameters<AddNodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        json_res(self.state.add_node(&a.descriptor_id, a.params, a.x, a.y))
    }

    /// Connect two nodes.
    #[tool(description = "Connect a source node's output port to a target node's input port (or promoted param).")]
    async fn connect(
        &self,
        Parameters(a): Parameters<ConnectArgs>,
    ) -> Result<CallToolResult, McpError> {
        json_res(self.state.connect(&a.source, &a.source_handle, &a.target, &a.target_handle))
    }

    /// Set a node parameter.
    #[tool(description = "Set a parameter value on a canvas node.")]
    async fn set_param(
        &self,
        Parameters(a): Parameters<SetParamArgs>,
    ) -> Result<CallToolResult, McpError> {
        json_res(self.state.set_param(&a.node_id, &a.name, a.value))
    }

    /// Remove a node (and its edges).
    #[tool(description = "Remove a node from the canvas along with its connected edges.")]
    async fn remove_node(
        &self,
        Parameters(a): Parameters<NodeIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        json_res(self.state.remove_node(&a.node_id))
    }

    /// Remove an edge.
    #[tool(description = "Remove a single edge from the canvas by its id.")]
    async fn remove_edge(
        &self,
        Parameters(a): Parameters<EdgeIdArgs>,
    ) -> Result<CallToolResult, McpError> {
        json_res(self.state.remove_edge(&a.edge_id))
    }

    /// Move a node.
    #[tool(description = "Move a node to a new canvas position (x, y).")]
    async fn move_node(
        &self,
        Parameters(a): Parameters<MoveNodeArgs>,
    ) -> Result<CallToolResult, McpError> {
        json_res(self.state.move_node(&a.node_id, a.x, a.y))
    }

    /// Save a workflow to disk.
    #[tool(description = "Save a workflow to a .lml/.json file the user can open in the GUI. Omit `snapshot` to save the current canvas.")]
    async fn save_workflow(
        &self,
        Parameters(a): Parameters<SaveWorkflowArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let out = tokio::task::spawn_blocking(move || state.save_workflow(&a.path, a.snapshot))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        json_res(out)
    }

    /// Load a workflow from disk.
    #[tool(description = "Load a workflow from a .lml/.json file. Set `apply`=true to also place it on the user's canvas.")]
    async fn load_workflow(
        &self,
        Parameters(a): Parameters<LoadWorkflowArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let out = tokio::task::spawn_blocking(move || state.load_workflow(&a.path, a.apply.unwrap_or(false)))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        json_res(out)
    }

    /// Save a composite (sub-graph) module.
    #[tool(description = "Save a composite (sub-graph) module so it appears as a reusable node in the palette.")]
    async fn save_composite_module(
        &self,
        Parameters(a): Parameters<ModuleArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let out = tokio::task::spawn_blocking(move || state.save_composite_module(a.module))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        json_res(out)
    }

    /// Save a script/program module.
    #[tool(description = "Save a script/external-program module so it appears as a reusable node in the palette.")]
    async fn save_script_module(
        &self,
        Parameters(a): Parameters<ModuleArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let out = tokio::task::spawn_blocking(move || state.save_script_module(a.module))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        json_res(out)
    }

    /// Generate a workflow from a natural-language task via the configured LLM.
    #[tool(description = "Ask the app's configured LLM to assemble a node graph from a task description. Set `apply`=true to place it on the canvas. (An MCP client can usually build graphs itself via list_nodes + set_canvas.)")]
    async fn generate_workflow(
        &self,
        Parameters(a): Parameters<GenerateWorkflowArgs>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        let out = tokio::task::spawn_blocking(move || state.generate_workflow(&a.prompt, a.apply.unwrap_or(false)))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        json_res(out)
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build());
        info.instructions = Some(INSTRUCTIONS.to_string());
        info
    }
}

/// Serve the MCP endpoint at `/mcp` (bearer-gated) on the given listener until
/// `cancel` fires.
pub async fn serve(state: McpState, listener: TcpListener, cancel: CancellationToken) {
    let token = state.token.clone();
    let svc_state = state.clone();
    let service = StreamableHttpService::new(
        move || Ok(McpServer::new(svc_state.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );
    let app = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn_with_state(
            token,
            crate::mcp::auth::require_bearer,
        ));
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(async move { cancel.cancelled().await })
        .await
    {
        eprintln!("[mcp] server error: {e}");
    }
}

/// Bind `addr` on the current runtime, returning the listener or the bind error.
pub async fn bind(addr: SocketAddr) -> std::io::Result<TcpListener> {
    TcpListener::bind(addr).await
}
