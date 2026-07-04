//! Transport-agnostic tool bodies. The pure logic lives in free functions (so it
//! can be unit-tested without a Tauri `AppHandle`); [`McpState`] methods just
//! supply the live registry/settings. `server.rs` wraps each into an rmcp
//! `#[tool]`.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::{json, Value};

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::composite::CompositeModule;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::model::SerializedGraph;
use misclab_core::graph::port::PortType;
use misclab_core::graph::script_node::ScriptModule;
use misclab_core::node::descriptor::{NodeDescriptor, ParamWidget};
use misclab_core::node::registry::NodeRegistry;
use misclab_core::node::{NodeEnv, PortMap};
use misclab_core::progress::NullSink;

use crate::mcp::io_adapt::{port_value_in, port_value_out};
use crate::mcp::state::{CanvasEdge, CanvasNode, CanvasSnapshot, McpState, Pos};

// ---- summarized node shape (keeps `list_nodes` payloads small) --------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PortInfo<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    port_type: PortType,
    required: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ParamInfo<'a> {
    name: &'a str,
    widget: &'a ParamWidget,
    default: &'a Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeSummary<'a> {
    id: &'a str,
    display_name: &'a str,
    category: &'a str,
    inputs: Vec<PortInfo<'a>>,
    outputs: Vec<PortInfo<'a>>,
    params: Vec<ParamInfo<'a>>,
}

fn port_infos(ps: &[misclab_core::node::descriptor::PortSpec]) -> Vec<PortInfo<'_>> {
    ps.iter()
        .map(|p| PortInfo { name: &p.name, port_type: p.port_type, required: p.required })
        .collect()
}

fn summarize(d: &NodeDescriptor) -> NodeSummary<'_> {
    NodeSummary {
        id: &d.id,
        display_name: &d.display_name,
        category: &d.category,
        inputs: port_infos(&d.inputs),
        outputs: port_infos(&d.outputs),
        params: d
            .params
            .iter()
            .map(|p| ParamInfo { name: &p.name, widget: &p.widget, default: &p.default })
            .collect(),
    }
}

// ---- pure logic (testable without a socket / AppHandle) ---------------------

/// Node-category counts — the cheapest first step for discovery.
pub(crate) fn list_categories_value(reg: &NodeRegistry) -> Value {
    use std::collections::BTreeMap;
    let descs = reg.descriptors();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for d in &descs {
        *counts.entry(d.category.clone()).or_insert(0) += 1;
    }
    let cats: Vec<Value> = counts
        .into_iter()
        .map(|(category, count)| json!({ "category": category, "count": count }))
        .collect();
    json!({ "total": descs.len(), "categories": cats })
}

/// Filter + shape the node catalog with three verbosity tiers (progressive
/// disclosure to save tokens): default = compact `{id,name,category}`,
/// `detail` = + ports/params, `full` = whole descriptors.
pub(crate) fn list_nodes_value(
    reg: &NodeRegistry,
    category: Option<&str>,
    query: Option<&str>,
    detail: bool,
    full: bool,
) -> Value {
    let all = reg.descriptors();
    let q = query.map(|s| s.to_lowercase());
    let filtered: Vec<&NodeDescriptor> = all
        .iter()
        .filter(|d| {
            let cat_ok = category.map(|c| d.category == c).unwrap_or(true);
            let q_ok = q
                .as_deref()
                .map(|q| d.id.to_lowercase().contains(q) || d.display_name.to_lowercase().contains(q))
                .unwrap_or(true);
            cat_ok && q_ok
        })
        .collect();
    let nodes = if full {
        filtered.iter().map(|d| serde_json::to_value(d).unwrap_or_default()).collect::<Vec<_>>()
    } else if detail {
        filtered.iter().map(|d| serde_json::to_value(summarize(d)).unwrap_or_default()).collect()
    } else {
        filtered
            .iter()
            .map(|d| json!({ "id": d.id, "name": d.display_name, "category": d.category }))
            .collect()
    };
    json!({ "count": nodes.len(), "nodes": nodes })
}

/// Clone settings to JSON with the two AI API keys redacted (so the AI can tell
/// whether a model is configured without seeing the secret).
pub(crate) fn redact_settings(env: &NodeEnv) -> Value {
    let mut v = serde_json::to_value(env).unwrap_or_default();
    for model in ["llm", "vision"] {
        if let Some(k) = v.pointer_mut(&format!("/ai/{model}/apiKey")) {
            let set = k.as_str().map(|s| !s.is_empty()).unwrap_or(false);
            *k = Value::String(if set { "***set***" } else { "" }.into());
        }
    }
    v
}

/// Some MCP clients serialize object/array-valued tool arguments as a JSON
/// *string* — our structured args (`inputs`, `params`, `graph`, `snapshot`) are
/// loosely-typed `serde_json::Value` with no schema `type`, so a client can't
/// tell they're meant to be objects and JSON-encodes them. Accept both forms: if
/// `v` is a string whose trimmed content parses as a JSON object/array, return
/// the parsed value; otherwise return it unchanged. Without this, such a client's
/// `run_node`/`run_graph`/`set_canvas` calls silently see empty inputs.
pub(crate) fn coerce_json_arg(v: &Value) -> Value {
    if let Value::String(s) = v {
        let t = s.trim();
        let looks_json = (t.starts_with('{') && t.ends_with('}'))
            || (t.starts_with('[') && t.ends_with(']'));
        if looks_json {
            if let Ok(parsed) = serde_json::from_str::<Value>(t) {
                return parsed;
            }
        }
    }
    v.clone()
}

// ---- McpState methods (supply live state) -----------------------------------

impl McpState {
    pub fn list_categories(&self) -> Value {
        list_categories_value(&self.combined_registry())
    }

    pub fn list_nodes(
        &self,
        category: Option<String>,
        query: Option<String>,
        detail: bool,
        full: bool,
    ) -> Value {
        list_nodes_value(&self.combined_registry(), category.as_deref(), query.as_deref(), detail, full)
    }

    pub fn describe_node(&self, id: &str) -> Result<Value, String> {
        let reg = self.combined_registry();
        match reg.get(id) {
            Some(e) => Ok(serde_json::to_value(&e.descriptor).unwrap_or_default()),
            None => Err(format!("unknown node id: {id}")),
        }
    }

    pub fn list_modules(&self) -> Value {
        let comps = self.composites.lock().expect("composites mutex poisoned").clone();
        let scripts = self.scripts.lock().expect("scripts mutex poisoned").clone();
        json!({ "composites": comps, "scripts": scripts })
    }

    pub fn get_settings_redacted(&self) -> Value {
        redact_settings(&self.settings.lock().expect("settings mutex poisoned"))
    }

    /// The user's current canvas as they see it (nodes + edges + rev).
    pub fn get_canvas(&self) -> Value {
        serde_json::to_value(&*self.canvas.lock().expect("canvas mutex poisoned")).unwrap_or_default()
    }

    /// Run a single node standalone. `inputs` maps port name → value JSON (see
    /// [`port_value_in`]); `params` is the node's param object. Blocking — call
    /// from a blocking thread.
    pub fn run_node(&self, descriptor_id: &str, inputs: &Value, params: &Value) -> Result<Value, String> {
        let inputs = coerce_json_arg(inputs);
        let mut port_inputs: PortMap = HashMap::new();
        if let Some(obj) = inputs.as_object() {
            for (k, v) in obj {
                port_inputs.insert(k.clone(), port_value_in(v, self)?);
            }
        }
        let params = coerce_json_arg(params);
        let params = if params.is_null() { json!({}) } else { params };
        let registry = self.combined_registry();
        let env = self.settings.lock().expect("settings mutex poisoned").clone();
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node_with_env(
            &registry,
            descriptor_id,
            &port_inputs,
            &params,
            &env,
            &NullSink,
            &cancel,
        )
        .map_err(|e| e.to_string())?;
        Ok(adapt_port_map(&out, self))
    }

    /// Run a whole graph and return per-node outputs. `graph` is a
    /// SerializedGraph ({nodes,edges}); if `None`, runs the current canvas.
    /// Blocking — call from a blocking thread.
    pub fn run_graph(&self, graph: Option<Value>) -> Result<Value, String> {
        let graph: SerializedGraph = match graph {
            Some(v) if !v.is_null() => {
                serde_json::from_value(coerce_json_arg(&v)).map_err(|e| format!("invalid graph: {e}"))?
            }
            _ => self.canvas.lock().expect("canvas mutex poisoned").to_serialized_graph(),
        };
        let registry = self.combined_registry();
        let env = self.settings.lock().expect("settings mutex poisoned").clone();
        let cancel = CancellationToken::new();
        let exec = GraphExecutor::new(&registry, &graph)
            .map_err(|e| e.to_string())?
            .with_env(env);
        let outputs = {
            let mut cache = self.cache.lock().expect("cache mutex poisoned");
            exec.run_with_cache(&NullSink, &cancel, &mut cache)
                .map_err(|e| e.to_string())?
        };
        let mut result = serde_json::Map::new();
        for (node_id, pm) in &outputs {
            result.insert(node_id.clone(), adapt_port_map(pm, self));
        }
        Ok(Value::Object(result))
    }
}

/// Apply [`port_value_out`] to every value in a port map → a JSON object.
fn adapt_port_map(pm: &PortMap, state: &McpState) -> Value {
    let mut out = serde_json::Map::new();
    for (port, v) in pm {
        out.insert(port.clone(), port_value_out(v, state));
    }
    Value::Object(out)
}

// ---- canvas mutations: pure ops on a snapshot (testable) --------------------

fn unique_node_id(cv: &CanvasSnapshot, descriptor_id: &str) -> String {
    // `ai_` prefix keeps AI ids from colliding with the frontend's counter.
    (1..).map(|i| format!("ai_{descriptor_id}_{i}")).find(|id| !cv.nodes.iter().any(|n| &n.id == id)).unwrap()
}

fn unique_edge_id(cv: &CanvasSnapshot) -> String {
    (1..).map(|i| format!("ai_edge_{i}")).find(|id| !cv.edges.iter().any(|e| &e.id == id)).unwrap()
}

pub(crate) fn add_node_to(
    cv: &mut CanvasSnapshot,
    descriptor_id: &str,
    label: String,
    color: String,
    params: Value,
    x: f64,
    y: f64,
) -> String {
    let id = unique_node_id(cv, descriptor_id);
    cv.nodes.push(CanvasNode {
        id: id.clone(),
        descriptor_id: descriptor_id.to_string(),
        label,
        color,
        params,
        input_params: Vec::new(),
        disabled: false,
        position: Pos { x, y },
    });
    id
}

/// Connect a source output to a target input (or param). Validates the handle
/// names against the node descriptors so the edge actually binds to real React
/// Flow handles — the #1 cause of AI-built edges that "don't connect" is a wrong
/// or unpromoted handle name. When the target handle is a *param* (not a declared
/// input port), it is promoted onto the node's `input_params` so the frontend
/// renders a handle for it. Unknown handles fail with the valid options listed,
/// so the caller can retry correctly instead of silently producing a dead edge.
pub(crate) fn connect_in(
    cv: &mut CanvasSnapshot,
    reg: &NodeRegistry,
    source: &str,
    sh: &str,
    target: &str,
    th: &str,
) -> Result<String, String> {
    let src_did = cv
        .nodes
        .iter()
        .find(|n| n.id == source)
        .map(|n| n.descriptor_id.clone())
        .ok_or_else(|| format!("unknown source node: {source}"))?;
    let tgt_did = cv
        .nodes
        .iter()
        .find(|n| n.id == target)
        .map(|n| n.descriptor_id.clone())
        .ok_or_else(|| format!("unknown target node: {target}"))?;

    // Validate the source output handle (skip if the descriptor is unknown, e.g.
    // a composite/script not in this registry — better to allow than to block).
    if let Some(entry) = reg.get(&src_did) {
        let d = &entry.descriptor;
        if !d.outputs.iter().any(|p| p.name == sh) {
            let valid: Vec<&str> = d.outputs.iter().map(|p| p.name.as_str()).collect();
            return Err(format!(
                "source node '{source}' ({src_did}) has no output port '{sh}'; valid outputs: {valid:?}"
            ));
        }
    }

    // Validate the target handle: a declared input port, or a param (which we
    // then promote to an input handle so it renders and accepts the edge).
    let mut promote_param = false;
    if let Some(entry) = reg.get(&tgt_did) {
        let d = &entry.descriptor;
        let is_input = d.inputs.iter().any(|p| p.name == th);
        let is_param = d.params.iter().any(|p| p.name == th);
        if !is_input && !is_param {
            let inputs: Vec<&str> = d.inputs.iter().map(|p| p.name.as_str()).collect();
            let params: Vec<&str> = d.params.iter().map(|p| p.name.as_str()).collect();
            return Err(format!(
                "target node '{target}' ({tgt_did}) has no input port or param '{th}'; \
                 valid inputs: {inputs:?}, params: {params:?}"
            ));
        }
        promote_param = !is_input && is_param;
    }
    if promote_param {
        if let Some(n) = cv.nodes.iter_mut().find(|n| n.id == target) {
            if !n.input_params.iter().any(|p| p == th) {
                n.input_params.push(th.to_string());
            }
        }
    }

    let id = unique_edge_id(cv);
    cv.edges.push(CanvasEdge {
        id: id.clone(),
        source: source.to_string(),
        source_handle: Some(sh.to_string()),
        target: target.to_string(),
        target_handle: Some(th.to_string()),
        edge_type: None,
    });
    Ok(id)
}

pub(crate) fn set_param_in(cv: &mut CanvasSnapshot, node_id: &str, name: &str, value: Value) -> Result<(), String> {
    let node = cv.nodes.iter_mut().find(|n| n.id == node_id).ok_or_else(|| format!("unknown node: {node_id}"))?;
    if !node.params.is_object() {
        node.params = json!({});
    }
    node.params.as_object_mut().unwrap().insert(name.to_string(), value);
    Ok(())
}

pub(crate) fn remove_node_in(cv: &mut CanvasSnapshot, node_id: &str) -> Result<(), String> {
    let before = cv.nodes.len();
    cv.nodes.retain(|n| n.id != node_id);
    if cv.nodes.len() == before {
        return Err(format!("unknown node: {node_id}"));
    }
    cv.edges.retain(|e| e.source != node_id && e.target != node_id);
    Ok(())
}

pub(crate) fn remove_edge_in(cv: &mut CanvasSnapshot, edge_id: &str) -> Result<(), String> {
    let before = cv.edges.len();
    cv.edges.retain(|e| e.id != edge_id);
    if cv.edges.len() == before {
        return Err(format!("unknown edge: {edge_id}"));
    }
    Ok(())
}

pub(crate) fn move_node_in(cv: &mut CanvasSnapshot, node_id: &str, x: f64, y: f64) -> Result<(), String> {
    let node = cv.nodes.iter_mut().find(|n| n.id == node_id).ok_or_else(|| format!("unknown node: {node_id}"))?;
    node.position = Pos { x, y };
    Ok(())
}

// ---- canvas mutations: McpState wrappers (bump rev + emit) -------------------

impl McpState {
    fn emit_canvas(&self, snapshot: &CanvasSnapshot) {
        self.app.emit_canvas(snapshot);
    }

    /// Lock the canvas, apply a fallible mutation, and — on success — bump `rev`
    /// and emit the new snapshot. The lock is released before the emit.
    fn mutate_canvas<T>(&self, f: impl FnOnce(&mut CanvasSnapshot) -> Result<T, String>) -> Result<T, String> {
        let (result, snap) = {
            let mut cv = self.canvas.lock().expect("canvas mutex poisoned");
            match f(&mut cv) {
                Ok(t) => {
                    cv.rev += 1;
                    (Ok(t), Some(cv.clone()))
                }
                Err(e) => (Err(e), None),
            }
        };
        if let Some(snap) = snap {
            self.emit_canvas(&snap);
        }
        result
    }

    /// Replace the whole canvas (server assigns a fresh `rev`).
    pub fn set_canvas(&self, snapshot: Value) -> Result<Value, String> {
        let incoming: CanvasSnapshot =
            serde_json::from_value(coerce_json_arg(&snapshot)).map_err(|e| format!("invalid snapshot: {e}"))?;
        let count = incoming.nodes.len();
        self.mutate_canvas(|cv| {
            cv.nodes = incoming.nodes;
            cv.edges = incoming.edges;
            Ok(())
        })?;
        Ok(json!({ "ok": true, "nodeCount": count }))
    }

    /// Add a node (params default from the descriptor, overridden by `params`).
    pub fn add_node(&self, descriptor_id: &str, params: Option<Value>, x: Option<f64>, y: Option<f64>) -> Result<Value, String> {
        let reg = self.combined_registry();
        let entry = reg.get(descriptor_id).ok_or_else(|| format!("unknown node id: {descriptor_id}"))?;
        let d = &entry.descriptor;
        let mut p = serde_json::Map::new();
        for ps in &d.params {
            p.insert(ps.name.clone(), ps.default.clone());
        }
        if let Some(pv) = params {
            if let Value::Object(over) = coerce_json_arg(&pv) {
                for (k, v) in over {
                    p.insert(k, v);
                }
            }
        }
        let (label, color) = (d.display_name.clone(), d.color.clone());
        let id = self.mutate_canvas(|cv| {
            Ok(add_node_to(cv, descriptor_id, label, color, Value::Object(p), x.unwrap_or(80.0), y.unwrap_or(80.0)))
        })?;
        Ok(json!({ "ok": true, "id": id }))
    }

    pub fn connect(&self, source: &str, source_handle: &str, target: &str, target_handle: &str) -> Result<Value, String> {
        let reg = self.combined_registry();
        let id = self.mutate_canvas(|cv| connect_in(cv, &reg, source, source_handle, target, target_handle))?;
        Ok(json!({ "ok": true, "edgeId": id }))
    }

    pub fn set_param(&self, node_id: &str, name: &str, value: Value) -> Result<Value, String> {
        self.mutate_canvas(|cv| set_param_in(cv, node_id, name, value))?;
        Ok(json!({ "ok": true }))
    }

    pub fn remove_node(&self, node_id: &str) -> Result<Value, String> {
        self.mutate_canvas(|cv| remove_node_in(cv, node_id))?;
        Ok(json!({ "ok": true }))
    }

    pub fn remove_edge(&self, edge_id: &str) -> Result<Value, String> {
        self.mutate_canvas(|cv| remove_edge_in(cv, edge_id))?;
        Ok(json!({ "ok": true }))
    }

    pub fn move_node(&self, node_id: &str, x: f64, y: f64) -> Result<Value, String> {
        self.mutate_canvas(|cv| move_node_in(cv, node_id, x, y))?;
        Ok(json!({ "ok": true }))
    }
}

// ---- persistence + AI --------------------------------------------------------

/// Only allow writing flow files (guards against clobbering arbitrary files).
fn workflow_path_ok(path: &str) -> bool {
    let p = path.to_lowercase();
    p.ends_with(".lml") || p.ends_with(".json")
}

fn file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("workflow")
        .to_string()
}

impl McpState {
    fn app_data_dir(&self) -> Result<std::path::PathBuf, String> {
        self.app.app_data_dir().ok_or_else(|| "app data dir unavailable".to_string())
    }

    /// Write the canvas (or a given snapshot) to a `.lml`/`.json` FlowProject file.
    pub fn save_workflow(&self, path: &str, snapshot: Option<Value>) -> Result<Value, String> {
        if !workflow_path_ok(path) {
            return Err("path must end with .lml or .json".into());
        }
        let snap: CanvasSnapshot = match snapshot {
            Some(v) if !v.is_null() => {
                serde_json::from_value(coerce_json_arg(&v)).map_err(|e| format!("invalid snapshot: {e}"))?
            }
            _ => self.canvas.lock().expect("canvas mutex poisoned").clone(),
        };
        let project = json!({
            "version": 1,
            "name": file_stem(path),
            "nodes": snap.nodes,
            "edges": snap.edges,
        });
        let text = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("write {path} failed: {e}"))?;
        Ok(json!({ "ok": true, "path": path, "nodeCount": snap.nodes.len() }))
    }

    /// Read a FlowProject `.lml`/`.json` into a CanvasSnapshot; if `apply`, also
    /// push it onto the user's canvas.
    pub fn load_workflow(&self, path: &str, apply: bool) -> Result<Value, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("read {path} failed: {e}"))?;
        let project: Value = serde_json::from_str(&text).map_err(|e| format!("parse {path} failed: {e}"))?;
        let snap = CanvasSnapshot {
            nodes: serde_json::from_value(project.get("nodes").cloned().unwrap_or_else(|| json!([])))
                .map_err(|e| format!("nodes: {e}"))?,
            edges: serde_json::from_value(project.get("edges").cloned().unwrap_or_else(|| json!([])))
                .map_err(|e| format!("edges: {e}"))?,
            rev: 0,
        };
        let value = serde_json::to_value(&snap).unwrap_or_default();
        if apply {
            self.mutate_canvas(|cv| {
                cv.nodes = snap.nodes;
                cv.edges = snap.edges;
                Ok(())
            })?;
        }
        Ok(value)
    }

    pub fn save_composite_module(&self, module: Value) -> Result<Value, String> {
        let m: CompositeModule = serde_json::from_value(module).map_err(|e| format!("invalid composite module: {e}"))?;
        let dir = self.app_data_dir()?;
        crate::modules::save_one(&dir, "modules", &m.id, &m).map_err(|e| e.to_string())?;
        let id = m.id.clone();
        let mut comps = self.composites.lock().expect("composites mutex poisoned");
        comps.retain(|x| x.id != id);
        comps.push(m);
        comps.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(json!({ "ok": true, "id": id }))
    }

    pub fn save_script_module(&self, module: Value) -> Result<Value, String> {
        let m: ScriptModule = serde_json::from_value(module).map_err(|e| format!("invalid script module: {e}"))?;
        let dir = self.app_data_dir()?;
        crate::modules::save_one(&dir, "script_modules", &m.id, &m).map_err(|e| e.to_string())?;
        let id = m.id.clone();
        let mut scripts = self.scripts.lock().expect("scripts mutex poisoned");
        scripts.retain(|x| x.id != id);
        scripts.push(m);
        scripts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(json!({ "ok": true, "id": id }))
    }

    /// Ask the configured LLM to assemble a graph from a task description. If
    /// `apply`, also push the result onto the canvas.
    pub fn generate_workflow(&self, prompt: &str, apply: bool) -> Result<Value, String> {
        if prompt.trim().is_empty() {
            return Err("prompt is empty".into());
        }
        let cfg = self.settings.lock().expect("settings mutex poisoned").ai.llm.clone();
        let gen = crate::commands::ai_workflow::generate(&self.combined_registry(), &cfg, prompt)
            .map_err(|e| e.message)?;
        let gen_value = serde_json::to_value(&gen).map_err(|e| e.to_string())?;
        if apply {
            let snap = self.generated_to_snapshot(&gen_value);
            let count = snap.nodes.len();
            self.mutate_canvas(|cv| {
                cv.nodes = snap.nodes;
                cv.edges = snap.edges;
                Ok(())
            })?;
            return Ok(json!({ "ok": true, "applied": true, "nodeCount": count, "graph": gen_value }));
        }
        Ok(gen_value)
    }

    /// Convert an AI-generated graph (GenNode/GenEdge JSON) into a CanvasSnapshot,
    /// filling label/color from the registry.
    fn generated_to_snapshot(&self, gen: &Value) -> CanvasSnapshot {
        let reg = self.combined_registry();
        let nodes = gen
            .get("nodes")
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|n| {
                        let did = n.get("descriptorId").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let (label, color) = reg
                            .get(&did)
                            .map(|e| (e.descriptor.display_name.clone(), e.descriptor.color.clone()))
                            .unwrap_or_else(|| (did.clone(), "#888888".into()));
                        CanvasNode {
                            id: n.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            descriptor_id: did,
                            label,
                            color,
                            params: n.get("params").cloned().unwrap_or_else(|| json!({})),
                            input_params: Vec::new(),
                            disabled: false,
                            position: Pos {
                                x: n.pointer("/position/x").and_then(|v| v.as_f64()).unwrap_or(80.0),
                                y: n.pointer("/position/y").and_then(|v| v.as_f64()).unwrap_or(80.0),
                            },
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let edges = gen
            .get("edges")
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .map(|(i, e)| CanvasEdge {
                        id: format!("gen_edge_{i}"),
                        source: e.pointer("/from/node").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        source_handle: e.pointer("/from/port").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        target: e.pointer("/to/node").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        target_handle: e.pointer("/to/port").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        edge_type: None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        CanvasSnapshot { nodes, edges, rev: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use misclab_core::nodes::default_registry;

    #[test]
    fn list_nodes_tiers_and_filters() {
        let reg = default_registry();

        // Default (compact): id + name + category only — no ports, no color.
        let all = list_nodes_value(&reg, None, None, false, false);
        assert_eq!(all["count"].as_u64().unwrap() as usize, reg.len());
        let first = &all["nodes"][0];
        assert!(first["id"].is_string() && first["name"].is_string());
        assert!(first.get("inputs").is_none(), "compact tier must omit ports");
        assert!(first.get("color").is_none());

        // Query filters by id/name substring and never exceeds the total.
        let b64 = list_nodes_value(&reg, None, Some("base64"), false, false);
        let n = b64["count"].as_u64().unwrap();
        assert!(n >= 1 && (n as usize) <= reg.len());
        for node in b64["nodes"].as_array().unwrap() {
            let id = node["id"].as_str().unwrap().to_lowercase();
            let name = node["name"].as_str().unwrap().to_lowercase();
            assert!(id.contains("base64") || name.contains("base64"));
        }

        // `detail` adds ports/params (but still no color).
        let detail = list_nodes_value(&reg, None, Some("base64"), true, false);
        assert!(detail["nodes"][0].get("inputs").is_some());
        assert!(detail["nodes"][0].get("color").is_none());

        // `full` returns whole descriptors (with color/cost).
        let full = list_nodes_value(&reg, None, Some("base64"), false, true);
        assert!(full["nodes"][0].get("color").is_some());
    }

    #[test]
    fn list_categories_counts_sum_to_total() {
        let reg = default_registry();
        let v = list_categories_value(&reg);
        assert_eq!(v["total"].as_u64().unwrap() as usize, reg.len());
        let sum: u64 = v["categories"].as_array().unwrap().iter().map(|c| c["count"].as_u64().unwrap()).sum();
        assert_eq!(sum, reg.len() as u64);
        // categories are non-empty and carry a name.
        assert!(v["categories"][0]["category"].is_string());
    }

    #[test]
    fn redact_settings_masks_api_keys() {
        let mut env = NodeEnv::default();
        env.ai.llm.api_key = "sk-secret".into();
        // vision key left empty
        let v = redact_settings(&env);
        assert_eq!(v["ai"]["llm"]["apiKey"], "***set***");
        assert_eq!(v["ai"]["vision"]["apiKey"], "");
        // non-secret fields survive.
        assert!(v["ai"]["llm"].get("baseUrl").is_some());
    }

    #[test]
    fn canvas_ops_add_connect_remove() {
        let reg = default_registry();
        let mut cv = CanvasSnapshot::default();

        // Add two nodes — ids are unique and `ai_`-prefixed.
        let a = add_node_to(&mut cv, "text_input", "In".into(), "#fff".into(), json!({}), 0.0, 0.0);
        let b = add_node_to(&mut cv, "text_output", "Out".into(), "#fff".into(), json!({}), 0.0, 0.0);
        assert_eq!(a, "ai_text_input_1");
        assert_eq!(cv.nodes.len(), 2);
        assert_ne!(a, b);

        // Connect them (handles are validated against the descriptors).
        let e = connect_in(&mut cv, &reg, &a, "text", &b, "text").unwrap();
        assert_eq!(cv.edges.len(), 1);
        assert_eq!(cv.edges[0].source_handle.as_deref(), Some("text"));

        // Connecting to a missing node is rejected and adds no edge.
        assert!(connect_in(&mut cv, &reg, &a, "text", "ghost", "text").is_err());
        assert_eq!(cv.edges.len(), 1);

        // A wrong source-handle name is rejected instead of making a dead edge.
        assert!(connect_in(&mut cv, &reg, &a, "nope", &b, "text").is_err());
        assert_eq!(cv.edges.len(), 1);

        // set_param merges into the node's params object.
        set_param_in(&mut cv, &a, "text", json!("hello")).unwrap();
        assert_eq!(cv.nodes[0].params["text"], "hello");

        // Removing a node also drops its edges (cascade).
        remove_node_in(&mut cv, &a).unwrap();
        assert_eq!(cv.nodes.len(), 1);
        assert!(cv.edges.is_empty(), "edge should cascade-delete with its node");

        // Removing an unknown edge/node errors.
        assert!(remove_edge_in(&mut cv, &e).is_err());
        assert!(remove_node_in(&mut cv, "ghost").is_err());

        // A fresh add reuses the now-free id slot.
        let a2 = add_node_to(&mut cv, "text_input", "In".into(), "#fff".into(), json!({}), 0.0, 0.0);
        assert_eq!(a2, "ai_text_input_1");
    }

    #[test]
    fn coerce_json_arg_accepts_stringified_objects() {
        // Real objects/arrays pass through unchanged.
        assert_eq!(coerce_json_arg(&json!({"text": "hi"})), json!({"text": "hi"}));
        assert_eq!(coerce_json_arg(&json!([1, 2, 3])), json!([1, 2, 3]));
        // A JSON-stringified object/array (what some MCP clients send) is parsed.
        assert_eq!(coerce_json_arg(&json!("{\"text\":\"hi\"}")), json!({"text": "hi"}));
        assert_eq!(coerce_json_arg(&json!("[1,2,3]")), json!([1, 2, 3]));
        // Plain scalar strings are left alone — not every string is JSON.
        assert_eq!(coerce_json_arg(&json!("RGBA")), json!("RGBA"));
        assert_eq!(coerce_json_arg(&json!("{ not json")), json!("{ not json"));
    }
}
