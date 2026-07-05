//! 故事模式（galgame）—— 把 CTF misc 解题过程演成一段视觉小说。
//!
//! 每一"幕"由 LLM 扮演解题搭档角色叙述当前局面并给出选项。搭档看【当前数据】、从
//! 真实节点目录里挑最对口的**单个**工具做成选项；玩家选中后，程序把当前数据直接喂给
//! 那一个节点跑（不再生成整张图），结果接力成下一轮的数据，搭档再据此反应、给下一步。

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

use misclab_core::ai::{self, ModelConfig};
use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::port::{PortType, PortValue};
use misclab_core::node::descriptor::NodeDescriptor;
use misclab_core::node::registry::NodeRegistry;
use misclab_core::node::{NodeEnv, PortMap};
use misclab_core::progress::NullSink;

use crate::commands::ai_workflow::truncate;
use crate::error::AppError;
use crate::state::{combined_registry_from, AppState};

// ---- wire types -------------------------------------------------------------

/// One trimmed past turn, kept by the frontend and echoed back for context.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    #[serde(default)]
    pub narration: String,
    #[serde(default)]
    pub picked: Option<String>,
    #[serde(default)]
    pub outputs: Option<String>,
}

/// The choice the player clicked. `node` present ⇒ run that one tool.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedChoice {
    #[serde(default)]
    pub text: String,
    /// Descriptor id of the tool to run on the current data.
    #[serde(default)]
    pub node: Option<String>,
    /// Params for that tool.
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// Request for one story step.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalgameStepRequest {
    /// The current working data (evolves each round; frontend chains it).
    #[serde(default)]
    pub challenge: String,
    #[serde(default)]
    pub challenge_kind: String,
    #[serde(default)]
    pub history: Vec<HistoryItem>,
    #[serde(default)]
    pub picked: Option<PickedChoice>,
}

/// A choice offered to the player. `node` ⇒ picking runs that one tool.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalgameChoice {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// One rendered story turn returned to the frontend.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalgameTurn {
    pub speaker: String,
    /// neutral | happy | thinking | worried | excited — drives sprite/background.
    pub mood: String,
    pub narration: String,
    pub choices: Vec<GalgameChoice>,
    /// The tool result summary produced this turn (feeds the next `history`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<String>,
    /// "good" (flag found) | "bad" (dead end) | absent (ongoing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending: Option<String>,
    /// Raw primary output of this round's tool — the frontend chains it into the
    /// next round's input so 套娃 decodes continue where they left off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_data: Option<String>,
}

// ---- what we parse back from the narrator LLM ------------------------------

#[derive(Deserialize)]
struct LlmChoice {
    #[serde(default)]
    text: String,
    /// The **exact node id** the narrator picked from the catalog (preferred).
    #[serde(default)]
    node: serde_json::Value,
    /// The operation the AI wants (a keyword phrase); the backend searches the
    /// tool list for a matching node. `tool` is kept only as a backwards-
    /// compatible alias because early prompts used that field name.
    #[serde(default)]
    operation: Option<String>,
    #[serde(default)]
    tool: serde_json::Value,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct LlmTurn {
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    mood: String,
    #[serde(default)]
    narration: String,
    #[serde(default)]
    choices: Vec<LlmChoice>,
    #[serde(default)]
    ending: Option<String>,
}

// ---- helpers ---------------------------------------------------------------

fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}

fn normalize_mood(m: &str) -> String {
    match m.trim() {
        "happy" | "excited" | "thinking" | "worried" | "neutral" => m.trim().to_string(),
        _ => "neutral".to_string(),
    }
}

fn is_story_tool_allowed(d: &NodeDescriptor) -> bool {
    d.category != "AI" && !d.id.starts_with("ai_")
}

/// A node story mode can actually run in a decode chain: story-allowed and its
/// primary input consumes the piped text/bytes (so `run_single_node` can feed the
/// current data straight in).
fn is_story_relevant(d: &NodeDescriptor) -> bool {
    is_story_tool_allowed(d)
        && d.inputs
            .first()
            .is_some_and(|i| matches!(i.port_type, PortType::Text | PortType::Bytes | PortType::Any))
}

/// Compact `id | 名称` catalog (grouped by category) of the tools the narrator may
/// pick from. The whole point: the narrator now *sees real ids and copies them
/// verbatim*, instead of naming a phrase we then fuzzy-search (which is how it
/// used to suggest “Hex 解码” yet run `to_hexdump`).
fn build_story_catalog(descriptors: &[NodeDescriptor]) -> String {
    let mut by_cat: BTreeMap<&str, Vec<&NodeDescriptor>> = BTreeMap::new();
    for d in descriptors.iter().filter(|d| is_story_relevant(d)) {
        by_cat.entry(d.category.as_str()).or_default().push(d);
    }
    let mut out = String::new();
    for (cat, ds) in &by_cat {
        out.push_str(&format!("# {cat}\n"));
        for d in ds {
            out.push_str(&format!("{} | {}\n", d.id, d.display_name));
        }
    }
    out
}

fn value_string(v: &serde_json::Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

fn parse_jsonish_value(s: &str) -> Option<serde_json::Value> {
    serde_json::from_str(s)
        .or_else(|_| serde_json::from_str(&strip_trailing_commas(s)))
        .ok()
}

fn operation_from_tool_value(v: &serde_json::Value) -> Option<String> {
    if let Some(s) = value_string(v) {
        return Some(s);
    }
    let obj = v.as_object()?;
    for key in ["operation", "query", "q", "tool"] {
        if let Some(s) = obj.get(key).and_then(value_string) {
            return Some(s);
        }
    }
    let args = obj
        .get("arguments")
        .or_else(|| obj.get("args"))
        .or_else(|| obj.get("input"));
    if let Some(args) = args {
        if let Some(s) = value_string(args).and_then(|s| {
            parse_jsonish_value(&s)
                .and_then(|v| operation_from_tool_value(&v))
                .or(Some(s))
        }) {
            return Some(s);
        }
        if let Some(s) = operation_from_tool_value(args) {
            return Some(s);
        }
    }
    obj.get("function")
        .and_then(|f| f.get("arguments"))
        .and_then(operation_from_tool_value)
}

fn choice_operation(c: &LlmChoice) -> Option<String> {
    c.operation
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| operation_from_tool_value(&c.tool))
}

/// Common operation phrases → confirmed node ids (query normalized to
/// alphanumerics only). This is the deterministic layer that kills the classic
/// "hex to text"/"hex decode" → `to_hexdump` mismatch.
fn alias_node(q_alnum: &str) -> Option<&'static str> {
    Some(match q_alnum {
        "hexdecode" | "hextotext" | "fromhex" | "hex2text" | "decodehex" | "hextostring" => "hex_decode",
        "base64decode" | "frombase64" | "b64decode" | "base64" | "b64" => "base64_decode",
        "base32decode" | "frombase32" | "base32" => "base32_decode",
        "urldecode" | "percentdecode" | "urldecodeonce" => "url_decode",
        "frombinary" | "binarytotext" | "bin2text" | "binarydecode" | "binary" => "from_binary",
        "morsedecode" | "frommorse" | "morse" | "morsecode" => "morse_decode",
        "rot13" => "rot13",
        "rot47" => "rot47",
        "atbash" => "atbash",
        "reverse" | "reversetext" | "reversestring" => "reverse",
        _ => return None,
    })
}

/// Rough encode/decode leaning of an operation phrase: +1 decode, -1 encode, 0
/// neutral. Note "X to text/ascii/string" reads as *decode* even though it says
/// "to" — that special-case is what the old fuzzy search got wrong.
fn op_intent(q: &str) -> i32 {
    let decode_to = ["to text", "to ascii", "to string", "to plain", "totext", "toascii"]
        .iter()
        .any(|k| q.contains(k));
    let mut s = 0i32;
    if decode_to
        || ["decode", "decrypt", "unescape", "unhex", "解码", "解密", "还原"]
            .iter()
            .any(|k| q.contains(k))
        || q.starts_with("from")
        || q.contains("from ")
    {
        s += 1;
    }
    if !decode_to
        && (["encode", "encrypt", "dump", "编码", "转成"].iter().any(|k| q.contains(k))
            || q.contains("to "))
    {
        s -= 1;
    }
    s.clamp(-1, 1)
}

/// Rough encode/decode direction of a node from its id: +1 decode-ish, -1
/// encode-ish, 0 symmetric/neutral.
fn node_direction(d: &NodeDescriptor) -> i32 {
    let id = d.id.as_str();
    if id.ends_with("_decode") || id.starts_with("from_") || id.contains("unescape") || id.contains("unhex") {
        1
    } else if id.ends_with("_encode") || id.starts_with("to_") || id.contains("dump") || id.contains("encrypt") {
        -1
    } else {
        0
    }
}

/// Search the tool list for a node matching the AI's operation query. Order:
/// (1) high-confidence alias / exact-id match, then (2) intent-aware token
/// scoring that penalizes picking an *encoder* when the ask is to *decode*. Only
/// used as a backstop now — the narrator normally supplies a real `node` id.
fn search_tools(descriptors: &[NodeDescriptor], query: &str) -> Option<String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return None;
    }
    let q_alnum: String = q.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let exists = |id: &str| descriptors.iter().any(|d| d.id == id && is_story_tool_allowed(d));

    // (1) Deterministic matches.
    if let Some(id) = alias_node(&q_alnum) {
        if exists(id) {
            return Some(id.to_string());
        }
    }
    let q_snake: String = q
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if exists(&q_snake) {
        return Some(q_snake);
    }

    // (2) Intent-aware fuzzy fallback.
    let intent = op_intent(&q);
    let tokens: Vec<String> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();
    if tokens.is_empty() {
        return None;
    }
    let mut best: Option<(i32, String)> = None;
    for d in descriptors {
        if !is_story_tool_allowed(d) {
            continue;
        }
        let id_spaced = d.id.replace('_', " ").to_lowercase();
        let hay = format!(
            "{} {} {}",
            id_spaced,
            d.display_name.to_lowercase(),
            d.description.to_lowercase()
        );
        let mut score = 0i32;
        for t in &tokens {
            if hay.contains(t.as_str()) {
                score += 1;
            }
            if id_spaced.split_whitespace().any(|w| w == t) {
                score += 2;
            }
        }
        if !q_alnum.is_empty() {
            let id_alnum: String = d.id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            if id_alnum == q_alnum {
                score += 8;
            }
        }
        // The crux: don't hand back an encoder/dumper when asked to decode.
        let dir = node_direction(d);
        if intent != 0 && dir != 0 {
            score += if intent == dir { 3 } else { -6 };
        }
        if score > 0 && best.as_ref().map_or(true, |(bs, _)| score > *bs) {
            best = Some((score, d.id.clone()));
        }
    }
    best.map(|(_, id)| id)
}

/// Resolve one choice to a real node id. Prefer the explicit `node` id the
/// narrator copied from the catalog; fall back to the legacy free-text
/// `operation`/`tool` search only when it didn't (or gave an unknown value).
fn resolve_choice_node(
    c: &LlmChoice,
    descriptors: &[NodeDescriptor],
    allowed: &HashSet<&str>,
) -> Option<String> {
    if let Some(id) = value_string(&c.node) {
        if allowed.contains(id.as_str()) {
            return Some(id);
        }
        if let Some(found) = search_tools(descriptors, &id) {
            return Some(found);
        }
    }
    choice_operation(c).and_then(|op| search_tools(descriptors, &op))
}

/// Map an LLM turn to the wire turn. Solving choices carry a real `node` id; if
/// the narrator offered too few actionable choices, top up from the backend's
/// heuristic hints so the player is never stuck with only chatter.
fn to_turn(llm: LlmTurn, descriptors: &[NodeDescriptor], hints: &[Hint]) -> GalgameTurn {
    let allowed: HashSet<&str> = descriptors
        .iter()
        .filter(|d| is_story_tool_allowed(d))
        .map(|d| d.id.as_str())
        .collect();
    let speaker = if llm.speaker.trim().is_empty() {
        "Misca".to_string()
    } else {
        llm.speaker
    };
    let mut choices: Vec<GalgameChoice> = llm
        .choices
        .into_iter()
        .filter(|c| !c.text.trim().is_empty())
        .map(|c| {
            let node = resolve_choice_node(&c, descriptors, &allowed);
            let params = if node.is_some() && c.params.is_object() {
                Some(c.params)
            } else {
                None
            };
            GalgameChoice {
                text: c.text,
                node,
                params,
            }
        })
        .take(4)
        .collect();
    let ending = match llm.ending.as_deref() {
        Some("good") => Some("good".to_string()),
        Some("bad") => Some("bad".to_string()),
        _ => None,
    };
    // Top up with backend-detected tools when the narrator under-delivered.
    if ending.is_none() && choices.iter().filter(|c| c.node.is_some()).count() < 2 {
        for h in hints {
            if choices.len() >= 4 {
                break;
            }
            if choices.iter().any(|c| c.node.as_deref() == Some(h.node)) {
                continue;
            }
            choices.push(GalgameChoice {
                text: h.label.to_string(),
                node: Some(h.node.to_string()),
                params: None,
            });
        }
    }
    if choices.is_empty() && ending.is_none() {
        choices.push(GalgameChoice {
            text: "继续".into(),
            node: None,
            params: None,
        });
    }
    GalgameTurn {
        speaker,
        mood: normalize_mood(&llm.mood),
        narration: llm.narration,
        choices,
        outputs: None,
        ending,
        result_data: None,
    }
}

fn parse_turn(raw: &str, descriptors: &[NodeDescriptor], hints: &[Hint]) -> Option<GalgameTurn> {
    let json = extract_json(raw)?;
    let llm: LlmTurn = serde_json::from_str(json)
        .or_else(|_| serde_json::from_str(&strip_trailing_commas(json)))
        .ok()?;
    if llm.narration.trim().is_empty() && llm.choices.is_empty() {
        return None;
    }
    Some(to_turn(llm, descriptors, hints))
}

/// Remove trailing commas before `}` / `]` — a common LLM JSON slip.
fn strip_trailing_commas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ',' {
            let mut ws = String::new();
            while let Some(&n) = chars.peek() {
                if n.is_whitespace() {
                    ws.push(n);
                    chars.next();
                } else {
                    break;
                }
            }
            let is_close = chars.peek().is_some_and(|&n| n == '}' || n == ']');
            if is_close {
                out.push_str(&ws); // drop the comma, keep whitespace
            } else {
                out.push(',');
                out.push_str(&ws);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn compact_sample(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .take(240)
        .collect::<String>()
}

fn looks_hex(s: &str) -> bool {
    let t = compact_sample(s);
    t.len() >= 8 && t.len() % 2 == 0 && t.chars().all(|c| c.is_ascii_hexdigit())
}

fn looks_binary(s: &str) -> bool {
    let t = compact_sample(s);
    t.len() >= 8 && t.chars().all(|c| matches!(c, '0' | '1'))
}

fn looks_base64(s: &str) -> bool {
    let t = compact_sample(s);
    t.len() >= 8
        && t.len() % 4 == 0
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '-' | '_'))
}

fn looks_morse(s: &str) -> bool {
    let t = s.trim();
    t.len() >= 5
        && t.chars()
            .all(|c| matches!(c, '.' | '-' | '/' | ' ' | '\n' | '\r' | '\t'))
        && (t.contains('.') || t.contains('-'))
}

fn looks_base32(s: &str) -> bool {
    let t = compact_sample(s);
    t.len() >= 8
        && t.chars().all(|c| matches!(c, 'A'..='Z' | '2'..='7' | '='))
        && t.chars().any(|c| c.is_ascii_uppercase())
}

/// One backend pre-judgement: a human label + the **exact** node id to run. Ids
/// here are hand-verified (never fuzzy-searched), so a hint labelled “十六进制转
/// 文本” always runs `hex_decode`, never `to_hexdump`.
struct Hint {
    label: &'static str,
    node: &'static str,
}

fn push_hint(hints: &mut Vec<Hint>, descriptors: &[NodeDescriptor], label: &'static str, node: &'static str) {
    let ok = descriptors.iter().any(|d| d.id == node && is_story_tool_allowed(d));
    if ok && !hints.iter().any(|h| h.node == node) {
        hints.push(Hint { label, node });
    }
}

/// Classify the current data by shape and map it to confirmed decode-node ids,
/// most-specific first. Drives both the prompt's 【后端预判】 grounding (so the
/// model spends less time guessing what the data is) and the choice top-up /
/// fallback (so a weak/failing model still yields correct actions).
fn detect_hints(descriptors: &[NodeDescriptor], data: &str) -> Vec<Hint> {
    let mut hints: Vec<Hint> = Vec::new();
    // URL escapes can coexist with any other layer, so offer it alongside.
    if data.contains('%') {
        push_hint(&mut hints, descriptors, "含 % 转义，先 URL 解码一层", "url_decode");
    }
    // The base/structural family is mutually exclusive — pick the most specific.
    if looks_morse(data) {
        push_hint(&mut hints, descriptors, "点划结构，按摩斯电码解码", "morse_decode");
    } else if looks_binary(data) {
        push_hint(&mut hints, descriptors, "全是 0/1，按二进制还原文本", "from_binary");
    } else if looks_hex(data) {
        push_hint(&mut hints, descriptors, "十六进制串，转成文本", "hex_decode");
    } else if looks_base32(data) {
        push_hint(&mut hints, descriptors, "像 Base32，解一层", "base32_decode");
    } else if looks_base64(data) {
        push_hint(&mut hints, descriptors, "像 Base64，解一层", "base64_decode");
    }
    hints
}

/// Used when the narrator LLM won't produce valid JSON — still shows the result
/// and offers a few real decode tools so the player never dead-ends.
fn fallback_turn(registry: &NodeRegistry, hints: &[Hint]) -> GalgameTurn {
    let descriptors = registry.descriptors();
    let mut choices: Vec<GalgameChoice> = Vec::new();
    // Confirmed, data-specific hints first.
    for h in hints {
        if choices.len() >= 3 {
            break;
        }
        choices.push(GalgameChoice {
            text: h.label.to_string(),
            node: Some(h.node.to_string()),
            params: None,
        });
    }
    // Then a few generically-useful decoders (all real ids; `find` guards them).
    let want = [
        "base64_decode",
        "base32_decode",
        "hex_decode",
        "url_decode",
        "from_binary",
        "morse_decode",
        "rot13",
        "reverse",
        "base58_decode",
        "ascii85_decode",
    ];
    for id in want {
        if choices.len() >= 3 {
            break;
        }
        if choices.iter().any(|c| c.node.as_deref() == Some(id)) {
            continue;
        }
        if let Some(d) = descriptors.iter().find(|d| d.id == id) {
            if !is_story_tool_allowed(d) {
                continue;
            }
            choices.push(GalgameChoice {
                text: format!("试试 {}", d.display_name),
                node: Some(d.id.clone()),
                params: None,
            });
        }
    }
    if choices.is_empty() {
        for d in descriptors
            .iter()
            .filter(|d| {
                is_story_tool_allowed(d)
                    && (d.category.contains("编码")
                        || d.category.contains("进制")
                        || d.category.contains("字符"))
            })
            .take(3)
        {
            choices.push(GalgameChoice {
                text: format!("试试 {}", d.display_name),
                node: Some(d.id.clone()),
                params: None,
            });
        }
    }
    choices.push(GalgameChoice {
        text: "重新观察一下".into(),
        node: None,
        params: None,
    });
    GalgameTurn {
        speaker: "Misca".into(),
        mood: "thinking".into(),
        narration: "我这边没拿到稳定的叙事 JSON，但后端已经按当前数据做了本地判断。先从这些具体工具里挑一个继续，不走 MCP。".into(),
        choices,
        outputs: None,
        ending: None,
        result_data: None,
    }
}

/// Summarize one typed port value into short human/LLM-readable text.
fn summarize_value(v: &PortValue) -> String {
    match v {
        PortValue::None => String::new(),
        PortValue::Text(s) => truncate(s.trim(), 600),
        PortValue::Number(n) => n.to_string(),
        PortValue::Bool(b) => b.to_string(),
        PortValue::Json(j) => truncate(&j.to_string(), 600),
        PortValue::StringList(v) => truncate(&v.join("\n"), 600),
        PortValue::Candidates(cs) => {
            let mut top = cs.clone();
            top.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let lines = top
                .iter()
                .take(5)
                .map(|c| format!("{}（{:.2}）", c.text, c.score))
                .collect::<Vec<_>>()
                .join("\n");
            truncate(&lines, 600)
        }
        PortValue::Bytes(b) => format!("<{} 字节二进制数据>", b.len()),
        PortValue::Artifact(_) => "<产物文件>".into(),
        PortValue::Image(_) => "<图片>".into(),
        PortValue::Fingerprint(_) => "<指纹>".into(),
    }
}

/// Best-effort text form of a value, for chaining into the next round.
fn value_to_text(v: &PortValue) -> Option<String> {
    match v {
        PortValue::Text(s) => Some(s.clone()),
        PortValue::Bytes(b) => Some(String::from_utf8_lossy(b).into_owned()),
        PortValue::Number(n) => Some(n.to_string()),
        PortValue::Bool(b) => Some(b.to_string()),
        PortValue::StringList(v) => Some(v.join("\n")),
        _ => None,
    }
}

/// Run one tool node on `work_data` and return (display summary, chained text).
fn run_single_node(
    registry: &NodeRegistry,
    env: &NodeEnv,
    descriptor_id: &str,
    params: &serde_json::Value,
    work_data: &str,
) -> (String, Option<String>) {
    let descriptors = registry.descriptors();
    let Some(desc) = descriptors.iter().find(|d| d.id == descriptor_id) else {
        return (format!("（找不到工具节点：{descriptor_id}）"), None);
    };
    if !is_story_tool_allowed(desc) {
        return (
            format!(
                "（{} 是 AI/元工具，故事模式不会嵌套调用；请改选一个具体解码/分析工具。）",
                desc.display_name
            ),
            None,
        );
    }

    // Feed the current data into the tool's first input port, typed to match.
    let mut inputs = PortMap::new();
    if let Some(inp) = desc.inputs.first() {
        let val = match inp.port_type {
            PortType::Bytes => PortValue::Bytes(work_data.as_bytes().to_vec().into()),
            _ => PortValue::Text(work_data.to_string()),
        };
        inputs.insert(inp.name.clone(), val);
    }

    let cancel = CancellationToken::new();
    match GraphExecutor::run_node_with_env(
        registry,
        descriptor_id,
        &inputs,
        params,
        env,
        &NullSink,
        &cancel,
    ) {
        Ok(pm) => {
            let mut parts = Vec::new();
            for o in &desc.outputs {
                if let Some(v) = pm.get(&o.name) {
                    let s = summarize_value(v);
                    if !s.trim().is_empty() {
                        parts.push(format!("[{}] {} → {}", desc.display_name, o.name, s));
                    }
                }
            }
            let summary = if parts.is_empty() {
                format!("[{}] 已执行（无文本输出）", desc.display_name)
            } else {
                truncate(&parts.join("\n"), 1500)
            };
            // Chain: the primary (first) output's text form.
            let primary = desc
                .outputs
                .first()
                .and_then(|o| pm.get(&o.name))
                .or_else(|| pm.values().next());
            let raw = primary
                .and_then(value_to_text)
                .map(|t| t.chars().take(100_000).collect());
            (summary, raw)
        }
        Err(e) => (format!("（{} 执行出错：{e}）", desc.display_name), None),
    }
}

// ---- prompt assembly -------------------------------------------------------

fn galgame_system(catalog: &str) -> String {
    format!(
        "你是 CTF misc 解题工具「LovelyMiscLab」里的视觉小说（galgame）叙事引擎，扮演解题搭档「Misca」——机灵、爱吐槽、很懂 misc、但很靠谱。\n\n\
玩家在解一道 misc 题。你的职责：看【当前数据】判断它是什么编码/密文，决定下一步该用哪个**工具节点**，把可尝试的操作做成选项让玩家选。\
你没有任何可调用工具；不要输出 MCP / function call / tool_calls / tools/search / list_nodes / run_node。只返回一个 JSON 对象。\
玩家选中带 node 的选项后，当前数据会自动喂给那个节点跑，结果接力给你。\n\n\
【可用工具节点】格式：id | 名称（按分类分组）。解题选项里的 node 只能填这里出现过的 id：\n{catalog}\n\
【怎么选对工具（关键，别再犯低级错）】\n\
- 解码/还原一律用 *_decode、from_* 这类节点；绝不要拿 *_encode、to_*、*_hexdump 这些“编码/转储”节点去解码。\n\
- 例：十六进制转文本用 hex_decode，不是 to_hexdump；二进制转文本用 from_binary，不是 to_binary；Base64 解码用 base64_decode。\n\
- 【后端预判】里给了按当前数据算出的建议 node id，优先采用；拿不准就先挑最像的那一个，用 run 后的结果再判断。\n\n\
【每个选项字段】\n\
- text：一句中文选项（像剧情分支，简短）。\n\
- node：可选。若这是一次解题动作，填上面目录里的**确切 id**（原样复制，别改写、别自造）。纯观察/对话就省略。\n\
- params：可选，对应该节点的参数对象。\n\n\
【规则】\n\
1. 一次只做一个操作。数据会一层层接力——套娃就一层层剥开。\n\
2. 先判断【当前数据】像什么（base64 / 十六进制 / url 编码 / 摩斯 / 二进制 / 各种进制 / 一段密文……），给 1~4 个最可能的操作作为选项，按可能性从高到低排序。\n\
3. 读【上一步结果】真实反应：解出新线索就惊喜，是乱码/走不通就沮丧，并给一个「换个方向」的选项。\n\
4. 若结果里已经出现明显的 flag（如 flag{{...}}），把 ending 设为 \"good\" 并让 Misca 庆祝通关。\n\
5. mood 从 neutral/happy/thinking/worried/excited 里选。\n\
6. narration 保持简短（1~3 句）。只输出一个 JSON 对象，禁止任何解释文字、markdown 代码块、MCP 调用、tool_calls 或函数调用包装。\n\n\
【输出格式】\n\
{{\"speaker\":\"Misca\",\"mood\":\"thinking\",\"narration\":\"……\",\"choices\":[{{\"text\":\"这串八成是 base64，解一层看看\",\"node\":\"base64_decode\"}},{{\"text\":\"再观察下有没有别的特征\"}}],\"ending\":null}}"
    )
}

fn build_context(req: &GalgameStepRequest, latest: Option<&str>, hints: &[Hint]) -> String {
    let mut s = String::new();
    let preview = truncate(req.challenge.trim(), 500);
    if preview.is_empty() {
        s.push_str("【当前数据】：（空）\n");
    } else {
        s.push_str(&format!("【当前数据】（可能已截断）：\n{preview}\n"));
    }
    if !req.challenge_kind.trim().is_empty() {
        s.push_str(&format!(
            "【数据类型】：{}\n",
            truncate(req.challenge_kind.trim(), 80)
        ));
    }
    if !hints.is_empty() {
        s.push_str("\n【后端预判】按当前数据特征，优先考虑这些工具（可直接把 node id 填进选项）：\n");
        for h in hints {
            s.push_str(&format!("- {}（node: {}）\n", h.label, h.node));
        }
    }

    let hist = &req.history;
    let start = hist.len().saturating_sub(8);
    if start < hist.len() {
        s.push_str("\n剧情回顾：\n");
        for h in &hist[start..] {
            if !h.narration.trim().is_empty() {
                s.push_str(&format!("- Misca：{}\n", truncate(h.narration.trim(), 200)));
            }
            if let Some(p) = h.picked.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
                s.push_str(&format!("  玩家选择：{p}\n"));
            }
            if let Some(o) = h
                .outputs
                .as_deref()
                .map(str::trim)
                .filter(|o| !o.is_empty())
            {
                s.push_str(&format!("  执行结果：{}\n", truncate(o, 300)));
            }
        }
    }

    if let Some(o) = latest.map(str::trim).filter(|o| !o.is_empty()) {
        s.push_str(&format!(
            "\n【上一步结果】（请据此真实反应）：\n{}\n",
            truncate(o, 1200)
        ));
    }
    if let Some(picked) = &req.picked {
        if !picked.text.trim().is_empty() {
            s.push_str(&format!(
                "\n【玩家刚刚选择】：{}\n",
                truncate(picked.text.trim(), 160)
            ));
        }
    }

    if req.picked.is_none() {
        s.push_str("\n现在开幕：讲讲这段数据给你的第一印象，判断它像什么，并给出最初 1~4 个下手工具作为选项。");
    } else {
        s.push_str("\n请根据以上进展继续叙述，并给出下一步 1~4 个选项。");
    }
    s
}

fn narrate(
    cfg: &ModelConfig,
    registry: &NodeRegistry,
    req: &GalgameStepRequest,
    latest: Option<&str>,
    hint_data: &str,
) -> Result<GalgameTurn, AppError> {
    let descriptors = registry.descriptors();
    // Give the narrator the real tool catalog so it picks verified ids, and a
    // heuristic pre-judgement of the current data so it spends less time guessing.
    let catalog = build_story_catalog(&descriptors);
    let hints = detect_hints(&descriptors, hint_data);
    let system = galgame_system(&catalog);
    let user = build_context(req, latest, &hints);

    let raw = ai::chat(cfg, &system, &user)?;
    if let Some(t) = parse_turn(&raw, &descriptors, &hints) {
        return Ok(t);
    }
    let system2 = format!("{system}\n\n重要：上一次回复不是合法 JSON。这次务必只输出一个 JSON 对象，narration 简短，不要任何多余文字或代码块。");
    if let Ok(raw2) = ai::chat(cfg, &system2, &user) {
        if let Some(t) = parse_turn(&raw2, &descriptors, &hints) {
            return Ok(t);
        }
    }
    Ok(fallback_turn(registry, &hints))
}

fn step(
    registry: &NodeRegistry,
    env: &NodeEnv,
    req: &GalgameStepRequest,
) -> Result<GalgameTurn, AppError> {
    let cfg = &env.ai.llm;
    if !cfg.is_configured() {
        return Err(AppError::new(
            "ai_config",
            "AI 文本模型未配置：请在「设置」里填写文本模型的 Base URL、模型名和 API Key。",
        ));
    }

    // 1. If the picked choice names a tool, run that single node on the current data.
    let mut outputs_summary: Option<String> = None;
    let mut result_data: Option<String> = None;
    if let Some(node_id) = req
        .picked
        .as_ref()
        .and_then(|p| p.node.as_deref())
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        let params = req
            .picked
            .as_ref()
            .and_then(|p| p.params.clone())
            .unwrap_or_else(|| json!({}));
        let (summary, raw) = run_single_node(registry, env, node_id, &params, &req.challenge);
        outputs_summary = Some(summary);
        result_data = raw;
    }

    // 2. Narrate the next turn, reacting to the fresh result. Detect on the *new*
    // data (this round's output) so the hints point at the next decode, not the
    // one we just did.
    let hint_data = result_data.clone().unwrap_or_else(|| req.challenge.clone());
    let mut turn = narrate(cfg, registry, req, outputs_summary.as_deref(), &hint_data)?;
    turn.outputs = outputs_summary;
    turn.result_data = result_data;
    Ok(turn)
}

/// One story step: optionally run one tool on the current data, then narrate.
#[tauri::command]
pub async fn galgame_step(
    state: State<'_, AppState>,
    req: GalgameStepRequest,
) -> Result<GalgameTurn, AppError> {
    let registry = {
        let comps = state.composites.lock().expect("composites mutex poisoned");
        let scripts = state.scripts.lock().expect("scripts mutex poisoned");
        combined_registry_from(state.registry.as_ref(), &comps, &scripts)
    };
    let env = state
        .settings
        .lock()
        .expect("settings mutex poisoned")
        .clone();
    tauri::async_runtime::spawn_blocking(move || step(&registry, &env, &req))
        .await
        .map_err(|e| AppError::new("join", e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use misclab_core::nodes::default_registry;

    fn descs() -> Vec<NodeDescriptor> {
        default_registry().descriptors()
    }

    /// The reported bug: the narrator meant "decode hex", the fuzzy search ran
    /// `to_hexdump`. Every phrasing must land on `hex_decode`, never an encoder.
    #[test]
    fn hex_decode_never_resolves_to_hexdump() {
        let d = descs();
        for q in ["hex decode", "hex to text", "from hex", "hex_decode", "十六进制解码"] {
            let got = search_tools(&d, q);
            assert_ne!(got.as_deref(), Some("to_hexdump"), "query {q:?} hit the encoder");
        }
        assert_eq!(search_tools(&d, "hex decode").as_deref(), Some("hex_decode"));
        assert_eq!(search_tools(&d, "hex to text").as_deref(), Some("hex_decode"));
    }

    #[test]
    fn decode_phrases_map_to_real_decoders() {
        let d = descs();
        let cases = [
            ("base64 decode", "base64_decode"),
            ("from base64", "base64_decode"),
            ("base32 decode", "base32_decode"),
            ("url decode", "url_decode"),
            ("from binary", "from_binary"),
            ("binary to text", "from_binary"),
            ("morse decode", "morse_decode"),
            ("rot13", "rot13"),
            ("reverse", "reverse"),
        ];
        for (q, want) in cases {
            assert_eq!(search_tools(&d, q).as_deref(), Some(want), "query {q:?}");
        }
    }

    /// A bare "to hex"/"to binary" ask is an *encoder* and must not be mistaken
    /// for the decoder — the intent penalty keeps the directions apart.
    #[test]
    fn encode_phrases_do_not_hit_decoders() {
        let d = descs();
        assert_ne!(search_tools(&d, "to binary").as_deref(), Some("from_binary"));
        assert_ne!(search_tools(&d, "to hex").as_deref(), Some("hex_decode"));
    }

    #[test]
    fn detect_hints_use_confirmed_ids() {
        let d = descs();
        let nodes = |data: &str| -> Vec<&'static str> {
            detect_hints(&d, data).into_iter().map(|h| h.node).collect()
        };
        assert!(nodes("48656c6c6f20776f726c6421").contains(&"hex_decode"));
        assert!(nodes("0100100001101001").contains(&"from_binary"));
        assert!(nodes("SGVsbG8gd29ybGQhIQ==").contains(&"base64_decode"));
        assert!(nodes(".... . .-.. .-.. ---").contains(&"morse_decode"));
        // A hex string must never suggest the hexdump encoder.
        assert!(!nodes("deadbeefdeadbeef").contains(&"to_hexdump"));
    }

    #[test]
    fn story_catalog_lists_decoders_and_excludes_ai() {
        let d = descs();
        let cat = build_story_catalog(&d);
        assert!(cat.contains("hex_decode |"));
        assert!(cat.contains("base64_decode |"));
        assert!(!cat.contains("ai_"), "AI/meta tools must not appear in the story catalog");
    }
}
