//! AI workflow generation. Given a natural-language task, ask the configured LLM
//! to assemble a node graph from the registry's catalog, validate it against the
//! real node signatures, auto-lay-it-out, and return a Template-shaped graph the
//! frontend loads directly.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use tauri::State;

use misclab_core::ai::{self, ModelConfig};
use misclab_core::graph::port::PortType;
use misclab_core::node::descriptor::{NodeDescriptor, ParamSpec, ParamWidget, PortSpec};
use misclab_core::node::registry::NodeRegistry;

use crate::error::AppError;
use crate::state::AppState;

// ---- output (matches the frontend Template node/edge shape) ----------------

#[derive(Serialize)]
pub(crate) struct Pos {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenNode {
    pub(crate) key: String,
    pub(crate) descriptor_id: String,
    pub(crate) params: serde_json::Value,
    pub(crate) position: Pos,
}

#[derive(Serialize)]
pub(crate) struct GenRef {
    pub(crate) node: String,
    pub(crate) port: String,
}

#[derive(Serialize)]
pub(crate) struct GenEdge {
    pub(crate) from: GenRef,
    pub(crate) to: GenRef,
}

#[derive(Serialize)]
pub struct GeneratedGraph {
    pub(crate) nodes: Vec<GenNode>,
    pub(crate) edges: Vec<GenEdge>,
    pub(crate) notes: String,
}

#[derive(Serialize)]
pub struct AiTextResult {
    text: String,
}

// ---- what we parse back from the LLM ---------------------------------------

#[derive(Deserialize)]
struct LlmNode {
    key: String,
    #[serde(rename = "type")]
    descriptor_id: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct LlmEdge {
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct LlmGraph {
    #[serde(default)]
    nodes: Vec<LlmNode>,
    #[serde(default)]
    edges: Vec<LlmEdge>,
    #[serde(default)]
    notes: String,
}

// ---- catalog ---------------------------------------------------------------

pub(crate) fn pt_str(t: PortType) -> &'static str {
    match t {
        PortType::Any => "any",
        PortType::Text => "text",
        PortType::Number => "number",
        PortType::Bool => "bool",
        PortType::Json => "json",
        PortType::StringList => "stringList",
        PortType::Candidates => "candidates",
        PortType::Bytes => "bytes",
        PortType::Artifact => "artifact",
        PortType::Image => "image",
        PortType::Fingerprint => "fingerprint",
    }
}

fn param_str(p: &ParamSpec) -> String {
    match &p.widget {
        ParamWidget::Select { options } => format!("{}=select[{}]", p.name, options.join("|")),
        ParamWidget::Toggle => format!("{}=toggle", p.name),
        ParamWidget::Number { .. } | ParamWidget::Slider { .. } => format!("{}=number", p.name),
        ParamWidget::File => format!("{}=file", p.name),
        ParamWidget::Image => format!("{}=image", p.name),
        ParamWidget::Text { .. } => format!("{}=text", p.name),
    }
}

fn ports_str(ports: &[PortSpec]) -> String {
    if ports.is_empty() {
        return "-".into();
    }
    ports
        .iter()
        .map(|p| format!("{}:{}", p.name, pt_str(p.port_type)))
        .collect::<Vec<_>>()
        .join(",")
}

fn catalog_line(d: &NodeDescriptor) -> String {
    let params = if d.params.is_empty() {
        "-".to_string()
    } else {
        d.params.iter().map(param_str).collect::<Vec<_>>().join(", ")
    };
    format!(
        "{} | {} | in:{} | out:{} | params:{}\n",
        d.id,
        d.display_name,
        ports_str(&d.inputs),
        ports_str(&d.outputs),
        params
    )
}

pub(crate) fn build_catalog(ds: &[NodeDescriptor]) -> String {
    ds.iter().map(catalog_line).collect()
}

/// Catalog text for only the descriptors whose id is in `only` (used by the
/// port-suggestion command to keep the prompt small and on-topic).
pub(crate) fn build_catalog_filtered(ds: &[NodeDescriptor], only: &[&str]) -> String {
    let set: std::collections::HashSet<&str> = only.iter().copied().collect();
    ds.iter()
        .filter(|d| set.contains(d.id.as_str()))
        .map(catalog_line)
        .collect()
}

fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}

pub(crate) fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

/// Resolve a `"key.port"` reference against known nodes; fill the default port if
/// omitted. `is_source` picks output vs input/param validation.
fn resolve(
    r: &str,
    descs: &HashMap<String, &NodeDescriptor>,
    is_source: bool,
) -> Option<(String, String)> {
    let (key, port) = match r.split_once('.') {
        Some((k, p)) => (k.trim().to_string(), Some(p.trim().to_string())),
        None => (r.trim().to_string(), None),
    };
    let d = descs.get(&key)?;
    let port = match port {
        Some(p) => {
            let ok = if is_source {
                d.outputs.iter().any(|o| o.name == p)
            } else {
                d.inputs.iter().any(|i| i.name == p) || d.params.iter().any(|pp| pp.name == p)
            };
            if ok {
                p
            } else {
                return None;
            }
        }
        None => {
            if is_source {
                d.outputs.first()?.name.clone()
            } else {
                d.inputs.first()?.name.clone()
            }
        }
    };
    Some((key, port))
}

/// Layered left-to-right layout: x by longest-path depth, y by order within a layer.
fn layout(keys: &[String], edges: &[(String, String)]) -> HashMap<String, Pos> {
    let mut level: HashMap<String, usize> = keys.iter().map(|k| (k.clone(), 0usize)).collect();
    for _ in 0..keys.len().max(1) {
        let mut changed = false;
        for (f, t) in edges {
            if let (Some(&lf), Some(&lt)) = (level.get(f), level.get(t)) {
                if lf + 1 > lt {
                    level.insert(t.clone(), lf + 1);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut by_level: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for k in keys {
        by_level.entry(*level.get(k).unwrap_or(&0)).or_default().push(k.clone());
    }
    let mut pos = HashMap::new();
    for (lvl, ks) in by_level {
        for (i, k) in ks.into_iter().enumerate() {
            pos.insert(k, Pos { x: 40.0 + lvl as f64 * 260.0, y: 60.0 + i as f64 * 150.0 });
        }
    }
    pos
}

pub(crate) fn generate(registry: &NodeRegistry, cfg: &ModelConfig, prompt: &str) -> Result<GeneratedGraph, AppError> {
    let descriptors = registry.descriptors();
    let catalog = build_catalog(&descriptors);

    let system = format!(
        "你是 LovelyMiscLab 的流程生成器。用户会描述一个 CTF misc 处理任务，你要用下列节点搭一个数据流图(DAG)来完成它。\n\n\
【可用节点】格式: id | 名称 | 输入端口 | 输出端口 | 参数\n{catalog}\n\
【规则】\n\
1. 只能使用上面出现过的节点 id。\n\
2. 用连线把上游【输出端口】连到下游【输入端口】，类型要匹配(text↔text, bytes↔bytes；any 可接任何类型)。\n\
3. 任务若从一段文本开始，用 text_input 作为源，并把该文本填进它的 text 参数。\n\
4. 需要展示最终文本结果时末尾接 text_output。\n\
5. select 参数只能取方括号里给出的选项之一。\n\
6. 参数也能被连线驱动：把某上游输出连到目标节点的【参数名】即可。\n\
7. 只输出一个 JSON 对象，禁止任何解释文字或 markdown 代码块。\n\n\
【输出格式】\n\
{{\"nodes\":[{{\"key\":\"in\",\"type\":\"text_input\",\"params\":{{\"text\":\"...\"}}}},{{\"key\":\"dec\",\"type\":\"base64_decode\"}},{{\"key\":\"out\",\"type\":\"text_output\"}}],\"edges\":[{{\"from\":\"in.text\",\"to\":\"dec.text\"}},{{\"from\":\"dec.text\",\"to\":\"out.text\"}}],\"notes\":\"一句话思路\"}}"
    );

    let raw = ai::chat(cfg, &system, prompt)?;
    let json = extract_json(&raw)
        .ok_or_else(|| AppError::new("ai_parse", format!("AI 未返回 JSON：{}", truncate(&raw, 200))))?;
    let llm: LlmGraph = serde_json::from_str(json)
        .map_err(|e| AppError::new("ai_parse", format!("解析 AI 结果失败: {e}")))?;

    let by_id: HashMap<&str, &NodeDescriptor> =
        descriptors.iter().map(|d| (d.id.as_str(), d)).collect();
    let mut node_desc: HashMap<String, &NodeDescriptor> = HashMap::new();
    let mut keys: Vec<String> = Vec::new();
    let mut raw_nodes: Vec<(String, String, serde_json::Value)> = Vec::new();
    let mut notes = llm.notes.clone();

    for n in &llm.nodes {
        match by_id.get(n.descriptor_id.as_str()) {
            Some(d) if !node_desc.contains_key(&n.key) => {
                node_desc.insert(n.key.clone(), d);
                keys.push(n.key.clone());
                let params = if n.params.is_object() {
                    n.params.clone()
                } else {
                    serde_json::json!({})
                };
                raw_nodes.push((n.key.clone(), n.descriptor_id.clone(), params));
            }
            Some(_) => {} // duplicate key — ignore
            None => notes.push_str(&format!("（忽略未知节点 {}）", n.descriptor_id)),
        }
    }
    if raw_nodes.is_empty() {
        return Err(AppError::new("ai_empty", "AI 未生成任何有效节点，请换个描述再试。"));
    }

    let mut edge_pairs: Vec<(String, String)> = Vec::new();
    let mut edges: Vec<GenEdge> = Vec::new();
    for e in &llm.edges {
        if let (Some((fk, fp)), Some((tk, tp))) =
            (resolve(&e.from, &node_desc, true), resolve(&e.to, &node_desc, false))
        {
            edge_pairs.push((fk.clone(), tk.clone()));
            edges.push(GenEdge {
                from: GenRef { node: fk, port: fp },
                to: GenRef { node: tk, port: tp },
            });
        }
    }

    let pos = layout(&keys, &edge_pairs);
    let nodes = raw_nodes
        .into_iter()
        .map(|(key, descriptor_id, params)| {
            let p = pos.get(&key).map(|p| Pos { x: p.x, y: p.y }).unwrap_or(Pos { x: 40.0, y: 60.0 });
            GenNode { key, descriptor_id, params, position: p }
        })
        .collect();

    Ok(GeneratedGraph { nodes, edges, notes })
}

fn graph_summary(registry: &NodeRegistry, graph: &misclab_core::graph::model::SerializedGraph) -> String {
    let descriptors = registry.descriptors();
    let by_id: HashMap<&str, &NodeDescriptor> =
        descriptors.iter().map(|d| (d.id.as_str(), d)).collect();
    let mut out = String::new();
    out.push_str("节点:\n");
    for n in &graph.nodes {
        let name = by_id
            .get(n.descriptor_id.as_str())
            .map(|d| d.display_name.as_str())
            .unwrap_or(n.descriptor_id.as_str());
        out.push_str(&format!(
            "- {}: {} ({}) params={}\n",
            n.id, name, n.descriptor_id, n.params
        ));
    }
    out.push_str("连线:\n");
    for e in &graph.edges {
        out.push_str(&format!(
            "- {}.{} -> {}.{}\n",
            e.from.node, e.from.port, e.to.node, e.to.port
        ));
    }
    out
}

#[tauri::command]
pub async fn generate_workflow(
    state: State<'_, AppState>,
    prompt: String,
) -> Result<GeneratedGraph, AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::new("ai_input", "请先描述你要做的任务。"));
    }
    let cfg = state.settings.lock().expect("settings mutex").ai.llm.clone();
    let registry = state.registry.clone();
    tauri::async_runtime::spawn_blocking(move || generate(&registry, &cfg, &prompt))
        .await
        .map_err(|e| AppError::new("join", e.to_string()))?
}

#[tauri::command]
pub async fn explain_workflow(
    state: State<'_, AppState>,
    graph: misclab_core::graph::model::SerializedGraph,
    prompt: String,
) -> Result<AiTextResult, AppError> {
    if graph.nodes.is_empty() {
        return Err(AppError::new("ai_input", "当前画布为空，无法解释流程。"));
    }
    let cfg = state.settings.lock().expect("settings mutex").ai.llm.clone();
    let registry = state.registry.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let summary = graph_summary(&registry, &graph);
        let system = "你是 LovelyMiscLab 的工作流讲解助手。请用中文解释节点图的数据流、关键参数、可能的失败点和下一步优化建议。回答要具体、简洁、可执行。";
        let user = if prompt.trim().is_empty() {
            format!("请解释这个工作流:\n{summary}")
        } else {
            format!("用户关注点: {}\n\n工作流:\n{summary}", prompt.trim())
        };
        let text = ai::chat(&cfg, system, &user)?;
        Ok(AiTextResult { text })
    })
    .await
    .map_err(|e| AppError::new("join", e.to_string()))?
}

// ---- port-hover next-node suggestions ---------------------------------------

/// The port type a param exposes when driven by a connection — mirrors the
/// frontend `paramPortType`.
pub(crate) fn param_port_type(w: &ParamWidget) -> PortType {
    match w {
        ParamWidget::Number { .. } | ParamWidget::Slider { .. } => PortType::Number,
        ParamWidget::Toggle => PortType::Bool,
        ParamWidget::Image => PortType::Image,
        _ => PortType::Text, // text / select / file
    }
}

/// Is `d` a sensible neighbour for a port of type `pt`? `dir_out` = the hovered
/// port is an output (we want a downstream consumer); otherwise it's an input
/// (we want an upstream producer).
fn port_compatible(d: &NodeDescriptor, pt: PortType, dir_out: bool) -> bool {
    if dir_out {
        d.inputs.iter().any(|i| i.port_type.accepts(pt))
            || d.params.iter().any(|p| param_port_type(&p.widget).accepts(pt))
    } else {
        d.outputs.iter().any(|o| pt.accepts(o.port_type))
    }
}

fn extract_json_array(s: &str) -> Option<&str> {
    let start = s.find('[')?;
    let end = s.rfind(']')?;
    (end > start).then(|| &s[start..=end])
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestCtx {
    descriptor_id: String,
    port: String,
    /// "in" or "out".
    direction: String,
    /// Serialized `PortType`, e.g. "text".
    port_type: String,
    #[serde(default)]
    hint: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    descriptor_id: String,
    #[serde(default)]
    reason: String,
}

fn suggest(registry: &NodeRegistry, cfg: &ModelConfig, ctx: &SuggestCtx) -> Result<Vec<Suggestion>, AppError> {
    let pt: PortType = serde_json::from_value(serde_json::Value::String(ctx.port_type.clone()))
        .unwrap_or(PortType::Any);
    let dir_out = ctx.direction != "in";
    let descriptors = registry.descriptors();
    let compatible_ids: Vec<&str> = descriptors
        .iter()
        .filter(|d| port_compatible(d, pt, dir_out))
        .map(|d| d.id.as_str())
        .collect();
    if compatible_ids.is_empty() {
        return Ok(vec![]);
    }
    let catalog = build_catalog_filtered(&descriptors, &compatible_ids);
    let dir_desc = if dir_out { "下游（消费该输出）" } else { "上游（产生该输入）" };
    let system = format!(
        "你是 LovelyMiscLab 的节点推荐助手。用户想在某节点的一个端口上接一个{dir_desc}节点。\n\n\
【候选节点】格式: id | 名称 | 输入 | 输出 | 参数\n{catalog}\n\
【要求】\n\
1. 只能从上面候选里选，按适用度从高到低排序，返回 3-6 个。\n\
2. 每个给一句极简中文理由（≤12字）。\n\
3. 只输出 JSON 数组：[{{\"descriptorId\":\"...\",\"reason\":\"...\"}}]，禁止任何其他文字或 markdown。"
    );
    let user = match ctx.hint.as_deref().map(str::trim) {
        Some(h) if !h.is_empty() => format!(
            "当前节点 {} 的端口「{}」(类型 {})。用户想做: {}",
            ctx.descriptor_id, ctx.port, ctx.port_type, h
        ),
        _ => format!(
            "当前节点 {} 的端口「{}」(类型 {})。给出最常见有用的下一步。",
            ctx.descriptor_id, ctx.port, ctx.port_type
        ),
    };
    let raw = ai::chat(cfg, &system, &user)?;
    let json = extract_json_array(&raw).unwrap_or(raw.as_str());
    let parsed: Vec<Suggestion> = serde_json::from_str(json).unwrap_or_default();
    // Drop any id the model invented that isn't in the compatible set.
    let allow: std::collections::HashSet<&str> = compatible_ids.into_iter().collect();
    Ok(parsed
        .into_iter()
        .filter(|s| allow.contains(s.descriptor_id.as_str()))
        .collect())
}

/// Suggest compatible next/previous nodes for a hovered port. Returns `Err` when
/// the LLM isn't configured — the frontend then just keeps its static candidate
/// list.
#[tauri::command]
pub async fn suggest_next_nodes(
    state: State<'_, AppState>,
    ctx: SuggestCtx,
) -> Result<Vec<Suggestion>, AppError> {
    let cfg = state.settings.lock().expect("settings mutex").ai.llm.clone();
    let registry = state.registry.clone();
    tauri::async_runtime::spawn_blocking(move || suggest(&registry, &cfg, &ctx))
        .await
        .map_err(|e| AppError::new("join", e.to_string()))?
}
