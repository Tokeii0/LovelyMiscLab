//! Agentic workflow builder. Instead of the one-shot `generate_workflow`, this
//! runs a tool-calling loop: the LLM calls `add_node` / `connect` / `set_param`
//! one step at a time and each step is streamed to the frontend over a Channel,
//! so the user watches the graph get built. Falls back to the one-shot generator
//! when the configured endpoint doesn't support function calling.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tauri::State;

use misclab_core::ai::{self, AssistantTurn, ModelConfig, ToolCall, ToolDef};
use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::{GraphExecutor, NodeCache};
use misclab_core::graph::model::{NodeInstance, PortRef, SerializedGraph};
use misclab_core::graph::port::{PortType, PortValue};
use misclab_core::node::descriptor::NodeDescriptor;
use misclab_core::node::registry::NodeRegistry;
use misclab_core::node::NodeEnv;
use misclab_core::progress::NullSink;

use crate::commands::ai_workflow::{build_catalog, generate, param_port_type, pt_str, truncate};
use crate::error::AppError;
use crate::state::AppState;

/// Primary ceiling: stop once the running transcript approaches the model's
/// context window. Modern models comfortably handle ~100k tokens, so let the
/// agent build as large as that allows. Tracked from each response's `usage`.
const MAX_CONTEXT_TOKENS: u64 = 100_000;
/// Runaway backstop on LLM turns (each turn may emit many tool calls) — the
/// effective bound only when the provider returns no token usage. Actual work is
/// bounded by NODES_MAX / RUN_BUDGET and the token budget above.
const STEPS_MAX: u32 = 64;
const NODES_MAX: usize = 40;
const RUN_BUDGET: u32 = 6;

/// One thing the agent did, streamed to the frontend and applied to the live
/// canvas. Nodes are referenced by the agent's own `key`; the frontend maps
/// those to real store ids.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AgentEvent {
    #[serde(rename_all = "camelCase")]
    Started { job: String, steps_max: u32 },
    Thinking { text: String },
    #[serde(rename_all = "camelCase")]
    AddNode {
        key: String,
        descriptor_id: String,
        /// Param *overrides* only; the frontend fills descriptor defaults.
        params: Value,
        /// One-line "巧思": why the agent adds this node now.
        reason: String,
    },
    #[serde(rename_all = "camelCase")]
    Connect {
        from_key: String,
        from_port: String,
        to_key: String,
        to_port: String,
        reason: String,
    },
    SetParam { key: String, name: String, value: Value, reason: String },
    RunStart { keys: Vec<String> },
    NodeResult { key: String, ok: bool, summary: String },
    ToolError { tool: String, message: String },
    #[serde(rename_all = "camelCase")]
    Done { notes: String, steps_used: u32 },
    Error { message: String },
}

// ---- the agent's authoritative (key-addressed) graph ------------------------

struct AgentNode {
    key: String,
    descriptor_id: String,
    params: Value,
}

struct AgentGraph {
    nodes: Vec<AgentNode>,
    edges: Vec<(String, String, String, String)>, // from_key, from_port, to_key, to_port
}

impl AgentGraph {
    fn has(&self, key: &str) -> bool {
        self.nodes.iter().any(|n| n.key == key)
    }
    fn descriptor_of<'a>(
        &self,
        key: &str,
        by_id: &'a HashMap<&str, &NodeDescriptor>,
    ) -> Option<&'a NodeDescriptor> {
        let n = self.nodes.iter().find(|n| n.key == key)?;
        by_id.get(n.descriptor_id.as_str()).copied()
    }
}

// ---- tool definitions -------------------------------------------------------

fn tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "add_node".into(),
            description:
                "新增一个节点。一次只加一个，加完随即把它连到已有的图上。key 是你自定义的唯一标识(后续连线用)，type 必须是目录里的节点 id。"
                    .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string", "description": "该节点的唯一标识(自定义, 如 in / dec / out)"},
                    "type": {"type": "string", "description": "节点 id, 必须来自可用节点目录"},
                    "params": {"type": "object", "description": "参数覆盖(可选)"},
                    "reason": {"type": "string", "description": "一句话巧思：结合线索说明为什么现在加这个节点(像在解题, ≤20字)"}
                },
                "required": ["key", "type", "reason"]
            }),
        },
        ToolDef {
            name: "connect".into(),
            description:
                "把上游输出端口连到下游输入端口/参数。from、to 形如 \"key.port\"；省略 \".port\" 用默认端口。"
                    .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "源, 形如 key 或 key.port"},
                    "to": {"type": "string", "description": "目标, 形如 key 或 key.port"},
                    "reason": {"type": "string", "description": "一句话巧思：这条连线在做什么(可选, ≤20字)"}
                },
                "required": ["from", "to"]
            }),
        },
        ToolDef {
            name: "set_param".into(),
            description: "设置某节点的一个参数值。".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string"},
                    "name": {"type": "string"},
                    "value": {"description": "参数值(字符串/数字/布尔)"},
                    "reason": {"type": "string", "description": "一句话巧思：为什么这么设(可选, ≤20字)"}
                },
                "required": ["key", "name", "value"]
            }),
        },
        ToolDef {
            name: "run_partial".into(),
            description:
                "运行当前已搭好的图(或指定的一部分)，查看每个节点的输出，用于验证/根据结果决定下一步。keys 省略则运行全部。"
                    .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "keys": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "要运行的节点 key 子集(可选)"
                    }
                }
            }),
        },
        ToolDef {
            name: "finish".into(),
            description: "流程搭建完成时调用，附一句话思路。".into(),
            parameters: json!({
                "type": "object",
                "properties": { "notes": {"type": "string", "description": "一句话思路"} }
            }),
        },
    ]
}

fn system_prompt(catalog: &str) -> String {
    format!(
        "你是 LovelyMiscLab 的画布搭建 Agent。用户会描述一个 CTF misc 任务，你要像**解题**一样把它拆成一步步，在画布上搭出数据流图(DAG)来完成它。\n\n\
【可用节点】格式: id | 名称 | 输入端口 | 输出端口 | 参数\n{catalog}\n\
【工作方式 —— 像解题一样推进，每步带巧思】\n\
1. 想清楚整条解题链路，再把节点依次搭出来：每个 add_node 都必须带 reason（一句话巧思，说明为什么加它，像在解题, ≤20字，例如“看着像套娃 Base64”），并 connect 到上游；connect / set_param 也尽量带 reason。\n\
2. 可以在一轮里连续加好几个节点和连线（应用会自动一个一个地动画呈现给用户），把该搭的一次搭到位，别把简单流程拖成很多轮。\n\
3. 拿不准下一步(如反复解码到出现 flag、验证某步是否正确)时，用 run_partial 运行当前图看输出，再据结果决定下一步。\n\
【硬性规则】\n\
4. 只能使用上面出现过的节点 id。\n\
5. 连线类型要匹配(text↔text, bytes↔bytes；any 通配)。\n\
6. 任务若从一段文本开始，用 text_input 作源并把文本填进它的 text 参数(add_node 的 params 或 set_param)；需要展示最终结果时末尾接 text_output。\n\
7. select 参数只能取候选值之一；参数也能被连线驱动(connect 到目标节点的“参数名”)。\n\
8. 只通过工具调用行动，不要输出散文解释。搭建完成后调用 finish。"
    )
}

// ---- tool dispatch ----------------------------------------------------------

enum Outcome {
    Continue,
    Finish(String),
}

fn tool_err(on_event: &Channel<AgentEvent>, tool: &str, msg: String) -> Value {
    let _ = on_event.send(AgentEvent::ToolError {
        tool: tool.into(),
        message: msg.clone(),
    });
    json!({ "error": msg })
}

fn arg_str<'a>(args: &'a Value, k: &str) -> &'a str {
    args.get(k).and_then(|v| v.as_str()).unwrap_or("").trim()
}

/// Resolve a `"key.port"` reference against the agent graph, filling the default
/// port when omitted. Returns `(key, port, type)` or a descriptive error message.
fn resolve_ref(
    graph: &AgentGraph,
    by_id: &HashMap<&str, &NodeDescriptor>,
    r: &str,
    is_source: bool,
) -> Result<(String, String, PortType), String> {
    let (key, port) = match r.split_once('.') {
        Some((k, p)) => (k.trim().to_string(), Some(p.trim().to_string())),
        None => (r.trim().to_string(), None),
    };
    if key.is_empty() {
        return Err("连线端点为空".into());
    }
    let d = graph
        .descriptor_of(&key, by_id)
        .ok_or_else(|| format!("未找到节点 key「{key}」(需先 add_node)"))?;
    match port {
        Some(p) => {
            if is_source {
                if let Some(o) = d.outputs.iter().find(|o| o.name == p) {
                    Ok((key, p, o.port_type))
                } else {
                    let opts: Vec<&str> = d.outputs.iter().map(|o| o.name.as_str()).collect();
                    Err(format!("节点「{key}」没有输出端口「{p}」；可选: {}", opts.join(", ")))
                }
            } else if let Some(i) = d.inputs.iter().find(|i| i.name == p) {
                Ok((key, p, i.port_type))
            } else if let Some(pp) = d.params.iter().find(|pp| pp.name == p) {
                Ok((key, p, param_port_type(&pp.widget)))
            } else {
                let mut opts: Vec<&str> = d.inputs.iter().map(|i| i.name.as_str()).collect();
                opts.extend(d.params.iter().map(|pp| pp.name.as_str()));
                Err(format!("节点「{key}」没有输入端口/参数「{p}」；可选: {}", opts.join(", ")))
            }
        }
        None => {
            if is_source {
                d.outputs
                    .first()
                    .map(|o| (key.clone(), o.name.clone(), o.port_type))
                    .ok_or_else(|| format!("节点「{key}」没有输出端口"))
            } else {
                d.inputs
                    .first()
                    .map(|i| (key.clone(), i.name.clone(), i.port_type))
                    .ok_or_else(|| format!("节点「{key}」没有输入端口"))
            }
        }
    }
}

/// Everything a tool needs beyond the graph. `runs_left` budgets run_partial to
/// keep token cost bounded.
struct AgentCtx<'a> {
    registry: &'a NodeRegistry,
    by_id: &'a HashMap<&'a str, &'a NodeDescriptor>,
    env: &'a NodeEnv,
    cache: &'a Mutex<NodeCache>,
    cancel: &'a CancellationToken,
    runs_left: u32,
}

fn preview_port(pv: &PortValue) -> String {
    match pv {
        PortValue::Text(s) => truncate(s, 120),
        PortValue::Number(n) => n.to_string(),
        PortValue::Bool(b) => b.to_string(),
        PortValue::StringList(v) => format!("{} 项", v.len()),
        PortValue::Candidates(v) => format!("{} 候选", v.len()),
        PortValue::Bytes(b) => format!("{} 字节", b.len()),
        PortValue::Json(v) => truncate(&v.to_string(), 120),
        PortValue::Image(_) => "图片".into(),
        PortValue::Artifact(_) => "文件".into(),
        PortValue::Fingerprint(_) => "指纹".into(),
        PortValue::None => "空".into(),
    }
}

/// Run the current graph (or a `keys` subset) through the executor, emit a
/// per-node result, and hand a compact summary back to the model so it can adapt.
fn tool_run_partial(
    graph: &AgentGraph,
    ctx: &mut AgentCtx,
    args: &Value,
    on_event: &Channel<AgentEvent>,
) -> Value {
    if ctx.runs_left == 0 {
        return tool_err(on_event, "run_partial", "运行次数已用尽".into());
    }
    if graph.nodes.is_empty() {
        return tool_err(on_event, "run_partial", "画布为空，先 add_node".into());
    }
    ctx.runs_left -= 1;

    let subset: Option<Vec<String>> = args
        .get("keys")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|k| k.as_str().map(str::to_string)).collect());
    let include: Vec<&AgentNode> = match &subset {
        Some(keys) => graph.nodes.iter().filter(|n| keys.contains(&n.key)).collect(),
        None => graph.nodes.iter().collect(),
    };
    if include.is_empty() {
        return tool_err(on_event, "run_partial", "没有匹配的节点".into());
    }
    let include_keys: std::collections::HashSet<&str> =
        include.iter().map(|n| n.key.as_str()).collect();

    let nodes: Vec<NodeInstance> = include
        .iter()
        .map(|n| NodeInstance {
            id: n.key.clone(),
            descriptor_id: n.descriptor_id.clone(),
            params: n.params.clone(),
            position: (0.0, 0.0),
        })
        .collect();
    let edges: Vec<misclab_core::graph::model::Edge> = graph
        .edges
        .iter()
        .filter(|(f, _, t, _)| include_keys.contains(f.as_str()) && include_keys.contains(t.as_str()))
        .map(|(f, fp, t, tp)| misclab_core::graph::model::Edge {
            from: PortRef { node: f.clone(), port: fp.clone() },
            to: PortRef { node: t.clone(), port: tp.clone() },
        })
        .collect();
    let sgraph = SerializedGraph { nodes, edges };

    let keys: Vec<String> = include.iter().map(|n| n.key.clone()).collect();
    let _ = on_event.send(AgentEvent::RunStart { keys });

    let outputs = match GraphExecutor::new(ctx.registry, &sgraph) {
        Ok(exec) => {
            let exec = exec.with_env(ctx.env.clone());
            let mut cache = ctx.cache.lock().expect("cache mutex poisoned");
            match exec.run_with_cache(&NullSink, ctx.cancel, &mut cache) {
                Ok(o) => o,
                Err(e) => return tool_err(on_event, "run_partial", format!("运行失败: {e}")),
            }
        }
        Err(e) => return tool_err(on_event, "run_partial", format!("构图失败: {e}")),
    };

    let mut results = serde_json::Map::new();
    for n in &include {
        let (ok, preview) = match outputs.get(&n.key) {
            Some(pm) => {
                let val = ctx
                    .by_id
                    .get(n.descriptor_id.as_str())
                    .and_then(|d| d.outputs.iter().find_map(|o| pm.get(&o.name)))
                    .or_else(|| pm.values().next());
                (true, val.map(preview_port).unwrap_or_else(|| "（无输出）".into()))
            }
            None => (false, "（未产出/失败）".into()),
        };
        let _ = on_event.send(AgentEvent::NodeResult {
            key: n.key.clone(),
            ok,
            summary: preview.clone(),
        });
        results.insert(n.key.clone(), json!({ "ok": ok, "preview": preview }));
    }
    json!({ "results": results })
}

fn dispatch(
    graph: &mut AgentGraph,
    ctx: &mut AgentCtx,
    call: &ToolCall,
    on_event: &Channel<AgentEvent>,
) -> (Outcome, Value) {
    let a = &call.arguments;
    let result = match call.name.as_str() {
        "add_node" => {
            let key = arg_str(a, "key").to_string();
            let ty = arg_str(a, "type").to_string();
            if key.is_empty() || ty.is_empty() {
                tool_err(on_event, "add_node", "缺少 key 或 type".into())
            } else if !ctx.by_id.contains_key(ty.as_str()) {
                tool_err(on_event, "add_node", format!("未知节点 id「{ty}」，只能用目录里的 id"))
            } else if graph.has(&key) {
                tool_err(on_event, "add_node", format!("key「{key}」已存在，请换一个"))
            } else if graph.nodes.len() >= NODES_MAX {
                tool_err(on_event, "add_node", format!("节点数量已达上限({NODES_MAX})"))
            } else {
                let params = a.get("params").filter(|v| v.is_object()).cloned().unwrap_or_else(|| json!({}));
                graph.nodes.push(AgentNode {
                    key: key.clone(),
                    descriptor_id: ty.clone(),
                    params: params.clone(),
                });
                let _ = on_event.send(AgentEvent::AddNode {
                    key: key.clone(),
                    descriptor_id: ty,
                    params,
                    reason: arg_str(a, "reason").to_string(),
                });
                json!({ "ok": true, "key": key })
            }
        }
        "connect" => {
            let from = arg_str(a, "from");
            let to = arg_str(a, "to");
            match (
                resolve_ref(graph, ctx.by_id, from, true),
                resolve_ref(graph, ctx.by_id, to, false),
            ) {
                (Ok((fk, fp, ft)), Ok((tk, tp, tt))) => {
                    if fk == tk {
                        tool_err(on_event, "connect", "不能把节点连到它自己".into())
                    } else if !tt.accepts(ft) {
                        tool_err(
                            on_event,
                            "connect",
                            format!(
                                "类型不匹配: {fk}.{fp}({}) → {tk}.{tp}({})",
                                pt_str(ft),
                                pt_str(tt)
                            ),
                        )
                    } else {
                        graph.edges.push((fk.clone(), fp.clone(), tk.clone(), tp.clone()));
                        let _ = on_event.send(AgentEvent::Connect {
                            from_key: fk,
                            from_port: fp,
                            to_key: tk,
                            to_port: tp,
                            reason: arg_str(a, "reason").to_string(),
                        });
                        json!({ "ok": true })
                    }
                }
                (Err(m), _) | (_, Err(m)) => tool_err(on_event, "connect", m),
            }
        }
        "set_param" => {
            let key = arg_str(a, "key").to_string();
            let name = arg_str(a, "name").to_string();
            let value = a.get("value").cloned().unwrap_or(Value::Null);
            if key.is_empty() || name.is_empty() {
                tool_err(on_event, "set_param", "缺少 key 或 name".into())
            } else if let Some(n) = graph.nodes.iter_mut().find(|n| n.key == key) {
                if !n.params.is_object() {
                    n.params = json!({});
                }
                if let Some(obj) = n.params.as_object_mut() {
                    obj.insert(name.clone(), value.clone());
                }
                let _ = on_event.send(AgentEvent::SetParam {
                    key,
                    name,
                    value,
                    reason: arg_str(a, "reason").to_string(),
                });
                json!({ "ok": true })
            } else {
                tool_err(on_event, "set_param", format!("未找到节点 key「{key}」"))
            }
        }
        "run_partial" => tool_run_partial(graph, ctx, a, on_event),
        "finish" => {
            let notes = a.get("notes").and_then(|v| v.as_str()).unwrap_or("完成").to_string();
            return (Outcome::Finish(notes), json!({ "ok": true }));
        }
        other => tool_err(on_event, other, format!("未知工具「{other}」")),
    };
    (Outcome::Continue, result)
}

// ---- one-shot fallback ------------------------------------------------------

/// When the endpoint can't do tool-calling, run the one-shot generator and
/// replay its graph as agent events so the frontend path is identical.
fn fallback(
    registry: &NodeRegistry,
    cfg: &ModelConfig,
    prompt: &str,
    on_event: &Channel<AgentEvent>,
    reason: &str,
) {
    let _ = on_event.send(AgentEvent::Thinking { text: reason.into() });
    match generate(registry, cfg, prompt) {
        Ok(g) => {
            for n in &g.nodes {
                let _ = on_event.send(AgentEvent::AddNode {
                    key: n.key.clone(),
                    descriptor_id: n.descriptor_id.clone(),
                    params: n.params.clone(),
                    reason: String::new(),
                });
            }
            for e in &g.edges {
                let _ = on_event.send(AgentEvent::Connect {
                    from_key: e.from.node.clone(),
                    from_port: e.from.port.clone(),
                    to_key: e.to.node.clone(),
                    to_port: e.to.port.clone(),
                    reason: String::new(),
                });
            }
            let _ = on_event.send(AgentEvent::Done {
                notes: format!("{}（已回退到一次性生成）", g.notes),
                steps_used: 0,
            });
        }
        Err(e) => {
            let _ = on_event.send(AgentEvent::Error { message: e.to_string() });
        }
    }
}

// ---- the loop ---------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn run_agent(
    registry: &NodeRegistry,
    cfg: &ModelConfig,
    env: &NodeEnv,
    cache: &Mutex<NodeCache>,
    prompt: &str,
    on_event: &Channel<AgentEvent>,
    job: &str,
    cancel: &CancellationToken,
) {
    let descriptors = registry.descriptors();
    let by_id: HashMap<&str, &NodeDescriptor> =
        descriptors.iter().map(|d| (d.id.as_str(), d)).collect();
    let catalog = build_catalog(&descriptors);
    let tools = tool_defs();
    let mut graph = AgentGraph { nodes: Vec::new(), edges: Vec::new() };
    let mut ctx = AgentCtx {
        registry,
        by_id: &by_id,
        env,
        cache,
        cancel,
        runs_left: RUN_BUDGET,
    };

    let mut messages: Vec<Value> = vec![
        json!({ "role": "system", "content": system_prompt(&catalog) }),
        json!({ "role": "user", "content": prompt }),
    ];

    let _ = on_event.send(AgentEvent::Started {
        job: job.into(),
        steps_max: STEPS_MAX,
    });

    for step in 0..STEPS_MAX {
        if cancel.is_cancelled() {
            let _ = on_event.send(AgentEvent::Error { message: "已取消".into() });
            return;
        }
        let (turn, usage) = match ai::chat_step(cfg, &messages, &tools) {
            Ok(t) => t,
            Err(e) => {
                if step == 0 {
                    fallback(registry, cfg, prompt, on_event, "当前模型不支持工具调用，已回退到一次性生成");
                } else {
                    let _ = on_event.send(AgentEvent::Error { message: e.to_string() });
                }
                return;
            }
        };
        match turn {
            AssistantTurn::Content(text) => {
                if step == 0 {
                    fallback(registry, cfg, prompt, on_event, "模型未使用工具，已回退到一次性生成");
                } else {
                    let notes = if text.trim().is_empty() { "完成".into() } else { text };
                    let _ = on_event.send(AgentEvent::Done { notes, steps_used: step });
                }
                return;
            }
            AssistantTurn::ToolCalls { raw_assistant_msg, calls } => {
                messages.push(raw_assistant_msg);
                for call in calls {
                    let (outcome, result) = dispatch(&mut graph, &mut ctx, &call, on_event);
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call.id,
                        "content": result.to_string(),
                    }));
                    if let Outcome::Finish(notes) = outcome {
                        let _ = on_event.send(AgentEvent::Done { notes, steps_used: step + 1 });
                        return;
                    }
                }
            }
        }
        // Primary ceiling: stop before the transcript outgrows the context window.
        // (total_tokens ≈ current transcript + last completion; the next turn's
        // prompt would be larger still.)
        if usage.total_tokens >= MAX_CONTEXT_TOKENS {
            let _ = on_event.send(AgentEvent::Done {
                notes: format!(
                    "已接近上下文上限（约 {} tokens），已尽力搭建",
                    usage.total_tokens
                ),
                steps_used: step + 1,
            });
            return;
        }
    }
    let _ = on_event.send(AgentEvent::Done {
        notes: "达到步数上限，已尽力搭建".into(),
        steps_used: STEPS_MAX,
    });
}

/// Build a workflow step-by-step, streaming each action to the canvas.
#[tauri::command]
pub async fn agent_run(
    state: State<'_, AppState>,
    prompt: String,
    on_event: Channel<AgentEvent>,
) -> Result<(), AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::new("ai_input", "请先描述你要做的任务。"));
    }
    let env = state.settings.lock().expect("settings mutex").clone();
    let cfg = env.ai.llm.clone();
    let registry = state.registry.clone();
    let cache = state.cache.clone();
    let cancel = CancellationToken::new();
    let job = state.jobs.start(cancel.clone());
    let job_for_run = job.clone();
    let res = tauri::async_runtime::spawn_blocking(move || {
        run_agent(&registry, &cfg, &env, &cache, &prompt, &on_event, &job_for_run, &cancel);
    })
    .await;
    state.jobs.finish(&job);
    res.map_err(|e| AppError::new("join", e.to_string()))
}
