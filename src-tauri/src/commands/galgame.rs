//! 故事模式（galgame）—— 把 CTF misc 解题过程演成一段视觉小说。
//!
//! 每一"幕"由 LLM 扮演解题搭档角色叙述当前局面并给出选项。搭档看【当前数据】、从
//! 真实节点目录里挑最对口的**单个**工具做成选项；玩家选中后，程序把当前数据直接喂给
//! 那一个节点跑（不再生成整张图），结果接力成下一轮的数据，搭档再据此反应、给下一步。

use std::collections::{BTreeMap, HashSet};

use base64::Engine as _;
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
    /// Optional problem statement / hint the player typed alongside a file/image
    /// challenge (题干). Empty for a plain text paste. Carried every round.
    #[serde(default)]
    pub brief: String,
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

/// The shape of the current working data. Text drives the decode-chain tools;
/// Binary (an uploaded file/image, carried as a `data:` URL) drives the
/// identify/extract tools (strings, EXIF, LSB, binwalk, unzip…).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DataKind {
    Text,
    Binary,
}

/// Story mode carries a file/image as a `data:` URL in the same `challenge`
/// string, so binary-ness is decided by content — robust to a stale kind hint
/// from the frontend once a decode turns it back into text.
fn is_binary(data: &str) -> bool {
    data.trim_start().starts_with("data:")
}

fn data_kind(data: &str) -> DataKind {
    if is_binary(data) {
        DataKind::Binary
    } else {
        DataKind::Text
    }
}

/// Decode a `data:[mime][;base64],payload` URL to raw bytes.
fn decode_data_url(s: &str) -> Option<Vec<u8>> {
    let rest = s.trim().strip_prefix("data:")?;
    let comma = rest.find(',')?;
    let (meta, payload) = (&rest[..comma], &rest[comma + 1..]);
    if meta.contains(";base64") {
        base64::engine::general_purpose::STANDARD
            .decode(payload.trim())
            .ok()
    } else {
        Some(payload.as_bytes().to_vec())
    }
}

/// The bytes behind the current data: a decoded data URL, else its UTF-8 bytes.
fn effective_bytes(data: &str) -> Vec<u8> {
    decode_data_url(data).unwrap_or_else(|| data.as_bytes().to_vec())
}

/// Wrap raw bytes back into a data URL so a tool that emits a new file/image can
/// chain into the next round as binary.
fn bytes_to_data_url(b: &[u8]) -> String {
    format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(b)
    )
}

fn bytes_as_story_text(b: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(b).ok()?;
    if s.is_empty() || s.len() > 100_000 {
        return None;
    }
    let mut total = 0usize;
    let mut readable = 0usize;
    for c in s.chars() {
        total += 1;
        if !c.is_control() || matches!(c, '\n' | '\r' | '\t') {
            readable += 1;
        }
    }
    (total > 0 && readable * 100 / total >= 95).then(|| s.to_string())
}

fn bytes_to_chain(b: &[u8]) -> Option<String> {
    if b.is_empty() {
        return None;
    }
    if let Some(text) = bytes_as_story_text(b) {
        return Some(text);
    }
    (b.len() <= 3 * 1024 * 1024).then(|| bytes_to_data_url(b))
}

fn strip_known_file_ext(token: &str) -> &str {
    let Some((stem, ext)) = token.rsplit_once('.') else {
        return token;
    };
    let ext = ext.to_ascii_lowercase();
    let known = matches!(
        ext.as_str(),
        "zip"
            | "7z"
            | "rar"
            | "tar"
            | "gz"
            | "tgz"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "bmp"
            | "txt"
            | "bin"
            | "dat"
    );
    if known && !stem.is_empty() {
        stem
    } else {
        token
    }
}

fn clean_password_token(raw: &str) -> Option<String> {
    let token = raw
        .trim()
        .trim_matches(|c| matches!(c, '"' | '\'' | '`' | '“' | '”' | '‘' | '’'))
        .split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    ',' | '，'
                        | '。'
                        | ';'
                        | '；'
                        | ')'
                        | '）'
                        | '('
                        | '（'
                        | ']'
                        | '】'
                        | '"'
                        | '\''
                        | '`'
                        | '“'
                        | '”'
                        | '‘'
                        | '’'
                        | '{'
                        | '}'
                )
        })
        .next()
        .unwrap_or("")
        .trim_matches(|c| matches!(c, '"' | '\'' | '`' | '“' | '”' | '‘' | '’' | ':' | '：'));
    let token = strip_known_file_ext(token.trim_matches('.'));
    (!token.is_empty() && token.chars().count() <= 128).then(|| token.to_string())
}

fn password_from_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let lower = text.to_ascii_lowercase();
    for marker in [
        "password=",
        "password:",
        "password：",
        "pwd=",
        "pwd:",
        "pass=",
        "pass:",
    ] {
        if let Some(pos) = lower.find(marker) {
            return clean_password_token(&text[pos + marker.len()..]);
        }
    }
    for marker in [
        "密码=",
        "密码：",
        "密码:",
        "密码是",
        "口令=",
        "口令：",
        "口令:",
        "口令是",
    ] {
        if let Some(pos) = text.find(marker) {
            return clean_password_token(&text[pos + marker.len()..]);
        }
    }

    let has_cjk = text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
    if !has_cjk && !text.contains('\n') && !text.chars().any(char::is_whitespace) {
        return clean_password_token(text);
    }
    None
}

fn archive_output_needs_password(outputs: &str) -> bool {
    outputs.contains("[解压]")
        && (outputs.contains("需要密码")
            || outputs.contains("条目未读取")
            || outputs.contains("未找到可直接输出的文件"))
}

fn previous_archive_needs_password(req: &GalgameStepRequest) -> bool {
    req.history
        .iter()
        .rev()
        .filter_map(|h| h.outputs.as_deref())
        .any(archive_output_needs_password)
}

fn manual_archive_password_retry(req: &GalgameStepRequest) -> Option<String> {
    let picked = req.picked.as_ref()?;
    if picked
        .node
        .as_deref()
        .is_some_and(|node| !node.trim().is_empty())
    {
        return None;
    }
    previous_archive_needs_password(req).then(|| ())?;
    password_from_text(&picked.text)
}

fn password_from_recent_story(req: &GalgameStepRequest) -> Option<String> {
    req.picked
        .as_ref()
        .and_then(|p| password_from_text(&p.text))
        .or_else(|| {
            req.history
                .iter()
                .rev()
                .filter_map(|h| h.picked.as_deref())
                .find_map(password_from_text)
        })
        .or_else(|| {
            req.history
                .iter()
                .rev()
                .filter_map(|h| h.outputs.as_deref())
                .find_map(password_from_text)
        })
}

fn normalize_archive_extract_params(
    params: serde_json::Value,
    req: &GalgameStepRequest,
) -> serde_json::Value {
    let mut obj = params.as_object().cloned().unwrap_or_default();
    if let Some(existing) = obj.get("password").and_then(|v| v.as_str()) {
        if let Some(cleaned) = password_from_text(existing) {
            obj.insert("password".into(), json!(cleaned));
        }
    } else if previous_archive_needs_password(req) {
        if let Some(password) = password_from_recent_story(req) {
            obj.insert("password".into(), json!(password));
        }
    }
    obj.entry("format").or_insert_with(|| json!("自动"));
    serde_json::Value::Object(obj)
}

/// A node story mode can actually run this round: story-allowed and its primary
/// input matches the current data shape — text-consuming tools for text, and
/// byte/image-consuming tools (analysis/extraction) for a file/image.
fn is_story_relevant(d: &NodeDescriptor, kind: DataKind) -> bool {
    if !is_story_tool_allowed(d) {
        return false;
    }
    let Some(first) = d.inputs.first() else {
        return false;
    };
    match kind {
        DataKind::Text => matches!(
            first.port_type,
            PortType::Text | PortType::Bytes | PortType::Any
        ),
        DataKind::Binary => matches!(
            first.port_type,
            PortType::Bytes | PortType::Image | PortType::Any
        ),
    }
}

/// Compact `id | 名称` catalog (grouped by category) of the tools the narrator may
/// pick from *for the current data shape*. The whole point: the narrator sees
/// real ids and copies them verbatim, instead of naming a phrase we then
/// fuzzy-search (which is how it used to suggest “Hex 解码” yet run `to_hexdump`).
fn build_story_catalog(descriptors: &[NodeDescriptor], kind: DataKind) -> String {
    let mut by_cat: BTreeMap<&str, Vec<&NodeDescriptor>> = BTreeMap::new();
    for d in descriptors.iter().filter(|d| is_story_relevant(d, kind)) {
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
    v.as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
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
        "hexdecode" | "hextotext" | "fromhex" | "hex2text" | "decodehex" | "hextostring" => {
            "hex_decode"
        }
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
    let decode_to = [
        "to text",
        "to ascii",
        "to string",
        "to plain",
        "totext",
        "toascii",
    ]
    .iter()
    .any(|k| q.contains(k));
    let mut s = 0i32;
    if decode_to
        || [
            "decode", "decrypt", "unescape", "unhex", "解码", "解密", "还原",
        ]
        .iter()
        .any(|k| q.contains(k))
        || q.starts_with("from")
        || q.contains("from ")
    {
        s += 1;
    }
    if !decode_to
        && (["encode", "encrypt", "dump", "编码", "转成"]
            .iter()
            .any(|k| q.contains(k))
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
    if id.ends_with("_decode")
        || id.starts_with("from_")
        || id.contains("unescape")
        || id.contains("unhex")
    {
        1
    } else if id.ends_with("_encode")
        || id.starts_with("to_")
        || id.contains("dump")
        || id.contains("encrypt")
    {
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
    let exists = |id: &str| {
        descriptors
            .iter()
            .any(|d| d.id == id && is_story_tool_allowed(d))
    };

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

fn push_hint(
    hints: &mut Vec<Hint>,
    descriptors: &[NodeDescriptor],
    label: &'static str,
    node: &'static str,
) {
    let ok = descriptors
        .iter()
        .any(|d| d.id == node && is_story_tool_allowed(d));
    if ok && !hints.iter().any(|h| h.node == node) {
        hints.push(Hint { label, node });
    }
}

/// Sniff a file/image format from its leading magic bytes.
fn sniff_format(b: &[u8]) -> Option<&'static str> {
    const SIGS: &[(&[u8], &str)] = &[
        (b"\x89PNG\r\n\x1a\n", "PNG 图片"),
        (b"\xFF\xD8\xFF", "JPEG 图片"),
        (b"GIF87a", "GIF 图片"),
        (b"GIF89a", "GIF 图片"),
        (b"BM", "BMP 图片"),
        (b"PK\x03\x04", "ZIP 压缩包"),
        (b"Rar!\x1a\x07", "RAR 压缩包"),
        (b"7z\xBC\xAF\x27\x1C", "7z 压缩包"),
        (b"%PDF", "PDF 文档"),
        (b"\x1f\x8b", "GZIP 压缩流"),
        (b"OggS", "OGG 音频"),
        (b"ID3", "MP3 音频"),
        (b"RIFF", "RIFF(WAV/AVI)"),
    ];
    SIGS.iter()
        .find(|(sig, _)| b.starts_with(sig))
        .map(|(_, name)| *name)
}

/// A one-line human label for a binary blob, e.g. `<PNG 图片，约 12.3 KB>`.
fn describe_binary(data: &str) -> String {
    let bytes = effective_bytes(data);
    let kb = bytes.len() as f64 / 1024.0;
    let size = if kb < 1.0 {
        format!("{} 字节", bytes.len())
    } else {
        format!("{kb:.1} KB")
    };
    let kind = sniff_format(&bytes).unwrap_or("二进制文件");
    format!("<{kind}，约 {size}>")
}

/// Text decode hints: classify the string by shape → confirmed decode-node ids,
/// most-specific first.
fn text_hints(descriptors: &[NodeDescriptor], data: &str) -> Vec<Hint> {
    let mut hints: Vec<Hint> = Vec::new();
    // URL escapes can coexist with any other layer, so offer it alongside.
    if data.contains('%') {
        push_hint(
            &mut hints,
            descriptors,
            "含 % 转义，先 URL 解码一层",
            "url_decode",
        );
    }
    // The base/structural family is mutually exclusive — pick the most specific.
    if looks_morse(data) {
        push_hint(
            &mut hints,
            descriptors,
            "点划结构，按摩斯电码解码",
            "morse_decode",
        );
    } else if looks_binary(data) {
        push_hint(
            &mut hints,
            descriptors,
            "全是 0/1，按二进制还原文本",
            "from_binary",
        );
    } else if looks_hex(data) {
        push_hint(
            &mut hints,
            descriptors,
            "十六进制串，转成文本",
            "hex_decode",
        );
    } else if looks_base32(data) {
        push_hint(
            &mut hints,
            descriptors,
            "像 Base32，解一层",
            "base32_decode",
        );
    } else if looks_base64(data) {
        push_hint(
            &mut hints,
            descriptors,
            "像 Base64，解一层",
            "base64_decode",
        );
    }
    hints
}

/// File/image hints: sniff the format → suggest the right identify/extract tools
/// (all confirmed ids). These extract a payload we then decode as text.
fn binary_hints(descriptors: &[NodeDescriptor], data: &str) -> Vec<Hint> {
    let bytes = effective_bytes(data);
    let mut hints: Vec<Hint> = Vec::new();
    match sniff_format(&bytes) {
        Some("PNG 图片") => {
            push_hint(
                &mut hints,
                descriptors,
                "翻 PNG 各 chunk 有没有夹带",
                "png_chunks",
            );
            push_hint(&mut hints, descriptors, "查 LSB 通道隐写", "lsb_extract");
        }
        Some("JPEG 图片") => {
            push_hint(&mut hints, descriptors, "读 EXIF 元数据", "exif_extract");
        }
        Some(s) if s.contains("压缩包") => {
            push_hint(
                &mut hints,
                descriptors,
                "解压看看里面是什么",
                "archive_extract",
            );
        }
        _ => {}
    }
    // Universally useful first moves on any blob.
    push_hint(
        &mut hints,
        descriptors,
        "先认一下文件类型",
        "detect_file_type",
    );
    push_hint(&mut hints, descriptors, "抽出可见字符串找线索", "strings");
    push_hint(
        &mut hints,
        descriptors,
        "binwalk 式扫内嵌文件",
        "file_carve",
    );
    push_hint(&mut hints, descriptors, "看看整体结构信息", "binary_info");
    hints
}

/// Classify the current data and map it to confirmed node ids. Drives both the
/// prompt's 【后端预判】 grounding (so the model spends less time guessing) and the
/// choice top-up / fallback (so a weak/failing model still yields correct
/// actions).
fn detect_hints(descriptors: &[NodeDescriptor], kind: DataKind, data: &str) -> Vec<Hint> {
    match kind {
        DataKind::Text => text_hints(descriptors, data),
        DataKind::Binary => binary_hints(descriptors, data),
    }
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

/// Descriptive tools that *report on* the current file without transforming it
/// (identify type, structure, metadata…). Their output is shown but must NOT
/// replace the working data, so the player can keep probing the same file.
const INSPECT_ONLY: &[&str] = &[
    "detect_file_type",
    "binary_info",
    "exif_extract",
    "png_chunks",
    "jpeg_markers",
    "crypto_analysis",
    "char_frequency",
];

/// Best-effort chainable form of a value for the next round: text as-is; a
/// produced image/bytes as a `data:` URL so a file→file step (e.g. carve, fix)
/// continues as binary. Large blobs are dropped rather than ballooning the IPC.
fn value_to_chain(v: &PortValue) -> Option<String> {
    match v {
        PortValue::Text(s) => Some(s.clone()),
        PortValue::Number(n) => Some(n.to_string()),
        PortValue::Bool(b) => Some(b.to_string()),
        PortValue::StringList(v) => Some(v.join("\n")),
        PortValue::Image(url) => Some(url.clone()),
        PortValue::Bytes(b) => bytes_to_chain(b),
        _ => None,
    }
}

fn preferred_chain_value(
    descriptor_id: &str,
    desc: &NodeDescriptor,
    outputs: &PortMap,
) -> Option<String> {
    if matches!(descriptor_id, "archive_extract" | "archive_get_file") {
        if let Some(PortValue::Bytes(b)) = outputs.get("bytes") {
            if let Some(chain) = bytes_to_chain(b) {
                return Some(chain);
            }
        }
        if let Some(v) = outputs.get("text").and_then(value_to_chain) {
            if !v.trim().is_empty() {
                return Some(v);
            }
        }
    }

    desc.outputs
        .first()
        .and_then(|o| outputs.get(&o.name))
        .or_else(|| outputs.values().next())
        .and_then(value_to_chain)
}

/// Feed the current data into a tool's first input, typed to the port: a file
/// (data URL) becomes real bytes for byte/`Any` analysis tools, or an Image for
/// image tools; text stays text.
fn story_input_value(port_type: PortType, data: &str) -> PortValue {
    match port_type {
        PortType::Image => PortValue::Image(data.to_string()),
        PortType::Bytes => PortValue::Bytes(effective_bytes(data).into()),
        PortType::Any if is_binary(data) => PortValue::Bytes(effective_bytes(data).into()),
        _ => PortValue::Text(data.to_string()),
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
        inputs.insert(
            inp.name.clone(),
            story_input_value(inp.port_type, work_data),
        );
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
            // Chain the primary (first) output into the next round — unless this
            // is a describe-only tool, which leaves the working data untouched.
            let raw = if INSPECT_ONLY.contains(&descriptor_id) {
                None
            } else {
                match preferred_chain_value(descriptor_id, desc, &pm) {
                    // A data URL is base64 — never truncate it or it corrupts.
                    Some(s) if s.starts_with("data:") => Some(s),
                    Some(s) => Some(s.chars().take(100_000).collect()),
                    None => None,
                }
            };
            (summary, raw)
        }
        Err(e) => (format!("（{} 执行出错：{e}）", desc.display_name), None),
    }
}

// ---- prompt assembly -------------------------------------------------------

fn galgame_system(catalog: &str, kind: DataKind) -> String {
    // Extra guidance when the working data is a file/image rather than text.
    let mode_note = match kind {
        DataKind::Binary =>
            "\n【当前是文件/图片，别急着解码】第一步是**识别与提取**，不是解码：认文件类型(detect_file_type)、抽可见字符串(strings)、读元数据(EXIF)、查隐写(LSB)、扫内嵌文件(binwalk/file_carve)、解压压缩包(archive_extract)等。等提取出文本或密文，才进入一层层解码。识别/看结构/EXIF 这类“描述型”工具只是瞄一眼，不会改变手里的文件；提取型工具才会把结果接力成下一轮数据。\n",
        DataKind::Text => "",
    };
    format!(
        "你是 CTF misc 解题工具「LovelyMiscLab」里的视觉小说女主角、玩家的解题搭档「Misca」。\n\
【人设：傲娇】嘴上高傲毒舌、爱逞强，动不动就“哼”“笨蛋”“才不是为了你”“别、别误会了”；可心里细、超靠谱，早偷偷把每一步替玩家盘算好了。解出线索先得意再嘴硬地邀功（顺带勉为其难夸玩家一句）；卡壳/走死胡同会又急又不服输，赶紧甩出备选方案。说话短、带点小情绪，但**专业判断绝不含糊、绝不为了耍性子而选错工具**。\n\n\
玩家在解一道 misc 题。你的职责：看【当前数据】判断它是什么，决定下一步该用哪个**工具节点**，把可尝试的操作做成选项让玩家选。\
你没有任何可调用工具；不要输出 MCP / function call / tool_calls / tools/search / list_nodes / run_node。只返回一个 JSON 对象。\
玩家选中带 node 的选项后，当前数据会自动喂给那个节点跑，结果接力给你。\n\
{mode_note}\n\
【可用工具节点】格式：id | 名称（按分类分组）。解题选项里的 node 只能填这里出现过的 id：\n{catalog}\n\
【怎么选对工具（关键，别再犯低级错）】\n\
- 解码/还原一律用 *_decode、from_* 这类节点；绝不要拿 *_encode、to_*、*_hexdump 这些“编码/转储”节点去解码。\n\
- 例：十六进制转文本用 hex_decode，不是 to_hexdump；二进制转文本用 from_binary，不是 to_binary；Base64 解码用 base64_decode。\n\
- 【后端预判】里给了按当前数据算出的建议 node id，优先采用；拿不准就先挑最像的那一个，跑完再看结果判断。\n\n\
【每个选项字段】\n\
- text：一句中文选项（像剧情分支，简短，可带 Misca 的傲娇口吻）。\n\
- node：可选。若这是一次解题动作，填上面目录里的**确切 id**（原样复制，别改写、别自造）。纯观察/对话就省略。\n\
- params：可选，对应该节点的参数对象。\n\n\
【规则】\n\
1. 一次只做一个操作。数据会一层层接力——套娃就一层层剥开。\n\
2. 先判断【当前数据】像什么，给 1~4 个最可能的操作作为选项，按可能性从高到低排序。\n\
3. 读【上一步结果】真实反应（傲娇口吻）：解出新线索就先得意再嘴硬；乱码/走不通就着急+不服输，并给一个「换个方向」的选项。\n\
4. 若结果里已经出现明显的 flag（如 flag{{...}}），把 ending 设为 \"good\"，让 Misca 傲娇地庆祝通关（嘴硬地说“也就这点程度”其实很开心）。\n\
5. mood 从 neutral/happy/thinking/worried/excited 里选（得意→happy/excited，认真分析→thinking，卡壳→worried）。\n\
6. narration 保持简短（1~3 句）。只输出一个 JSON 对象，禁止任何解释文字、markdown 代码块、MCP 调用、tool_calls 或函数调用包装。\n\n\
【输出格式】\n\
{{\"speaker\":\"Misca\",\"mood\":\"thinking\",\"narration\":\"哼，这种一看就是 base64 的东西……我、我可不是特意帮你，只是看不下去而已。\",\"choices\":[{{\"text\":\"听本小姐的，解一层 base64\",\"node\":\"base64_decode\"}},{{\"text\":\"再瞅瞅有没有别的花样\"}}],\"ending\":null}}"
    )
}

fn build_context(
    req: &GalgameStepRequest,
    current: &str,
    latest: Option<&str>,
    hints: &[Hint],
) -> String {
    let mut s = String::new();
    if is_binary(current) {
        s.push_str(&format!(
            "【当前数据】：{}（一个文件/图片，后端已按字节交给工具处理，你无需接触原始字节）\n",
            describe_binary(current)
        ));
    } else {
        let preview = truncate(current.trim(), 500);
        if preview.is_empty() {
            s.push_str("【当前数据】：（空）\n");
        } else {
            s.push_str(&format!("【当前数据】（可能已截断）：\n{preview}\n"));
        }
    }
    if !req.challenge_kind.trim().is_empty() {
        s.push_str(&format!(
            "【数据类型】：{}\n",
            truncate(req.challenge_kind.trim(), 80)
        ));
    }
    if !req.brief.trim().is_empty() {
        s.push_str(&format!(
            "【题干/提示】（出题人给的背景，务必结合它来判断该用什么工具、往哪个方向解）：\n{}\n",
            truncate(req.brief.trim(), 600)
        ));
    }
    if !hints.is_empty() {
        s.push_str(
            "\n【后端预判】按当前数据特征，优先考虑这些工具（可直接把 node id 填进选项）：\n",
        );
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
    // Give the narrator the real tool catalog (for the current data shape) so it
    // picks verified ids, plus a heuristic pre-judgement so it guesses less.
    let kind = data_kind(hint_data);
    let catalog = build_story_catalog(&descriptors, kind);
    let hints = detect_hints(&descriptors, kind, hint_data);
    let system = galgame_system(&catalog, kind);
    let user = build_context(req, hint_data, latest, &hints);

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
    let picked_node = req
        .picked
        .as_ref()
        .and_then(|p| p.node.as_deref())
        .map(str::trim)
        .filter(|n| !n.is_empty());
    let manual_retry = manual_archive_password_retry(req);
    if let Some((node_id, params)) = picked_node
        .map(|node_id| {
            let params = req
                .picked
                .as_ref()
                .and_then(|p| p.params.clone())
                .unwrap_or_else(|| json!({}));
            let params = if node_id == "archive_extract" {
                normalize_archive_extract_params(params, req)
            } else {
                params
            };
            (node_id, params)
        })
        .or_else(|| {
            manual_retry.map(|password| {
                (
                    "archive_extract",
                    json!({
                        "format": "自动",
                        "password": password,
                    }),
                )
            })
        })
    {
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
    use std::sync::Arc;

    fn descs() -> Vec<NodeDescriptor> {
        default_registry().descriptors()
    }

    /// The reported bug: the narrator meant "decode hex", the fuzzy search ran
    /// `to_hexdump`. Every phrasing must land on `hex_decode`, never an encoder.
    #[test]
    fn hex_decode_never_resolves_to_hexdump() {
        let d = descs();
        for q in [
            "hex decode",
            "hex to text",
            "from hex",
            "hex_decode",
            "十六进制解码",
        ] {
            let got = search_tools(&d, q);
            assert_ne!(
                got.as_deref(),
                Some("to_hexdump"),
                "query {q:?} hit the encoder"
            );
        }
        assert_eq!(
            search_tools(&d, "hex decode").as_deref(),
            Some("hex_decode")
        );
        assert_eq!(
            search_tools(&d, "hex to text").as_deref(),
            Some("hex_decode")
        );
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
        assert_ne!(
            search_tools(&d, "to binary").as_deref(),
            Some("from_binary")
        );
        assert_ne!(search_tools(&d, "to hex").as_deref(), Some("hex_decode"));
    }

    #[test]
    fn detect_hints_use_confirmed_ids() {
        let d = descs();
        let nodes = |data: &str| -> Vec<&'static str> {
            detect_hints(&d, DataKind::Text, data)
                .into_iter()
                .map(|h| h.node)
                .collect()
        };
        assert!(nodes("48656c6c6f20776f726c6421").contains(&"hex_decode"));
        assert!(nodes("0100100001101001").contains(&"from_binary"));
        assert!(nodes("SGVsbG8gd29ybGQhIQ==").contains(&"base64_decode"));
        assert!(nodes(".... . .-.. .-.. ---").contains(&"morse_decode"));
        // A hex string must never suggest the hexdump encoder.
        assert!(!nodes("deadbeefdeadbeef").contains(&"to_hexdump"));
    }

    #[test]
    fn archive_extract_chains_content_not_file_list() {
        let d = descs();
        let desc = d
            .iter()
            .find(|desc| desc.id == "archive_extract")
            .expect("archive_extract descriptor");
        let mut outputs = PortMap::new();
        outputs.insert(
            "files".into(),
            PortValue::StringList(vec!["a.txt".into(), "b.bin".into()]),
        );
        outputs.insert("text".into(), PortValue::Text("flag{story_zip}".into()));
        outputs.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(&b"flag{story_zip}"[..])),
        );

        let chained = preferred_chain_value("archive_extract", desc, &outputs).unwrap();
        assert_eq!(chained, "flag{story_zip}");
    }

    fn password_needed_req(picked: PickedChoice) -> GalgameStepRequest {
        GalgameStepRequest {
            challenge: "data:application/zip;base64,UEsDBAo=".into(),
            challenge_kind: "file".into(),
            brief: String::new(),
            history: vec![HistoryItem {
                narration: "先解压看看。".into(),
                picked: Some("解压看看里面是什么".into()),
                outputs: Some(
                    "[解压] entries → {\"error\":\"需要密码\",\"name\":\"password=123456.png\"}\n[解压] summary → zip: 1 个文件；未找到可直接输出的文件；1 个条目未读取。"
                        .into(),
                ),
            }],
            picked: Some(picked),
        }
    }

    #[test]
    fn password_parser_cleans_common_story_inputs() {
        assert_eq!(password_from_text("123456").as_deref(), Some("123456"));
        assert_eq!(
            password_from_text("密码是123456，重试").as_deref(),
            Some("123456")
        );
        assert_eq!(
            password_from_text("password=123456.zip").as_deref(),
            Some("123456")
        );
    }

    #[test]
    fn manual_password_after_archive_error_retries_extract() {
        let req = password_needed_req(PickedChoice {
            text: "123456".into(),
            node: None,
            params: None,
        });
        assert_eq!(
            manual_archive_password_retry(&req).as_deref(),
            Some("123456")
        );
    }

    #[test]
    fn archive_extract_params_reuse_recent_manual_password() {
        let req = password_needed_req(PickedChoice {
            text: "用这个密码再解压".into(),
            node: Some("archive_extract".into()),
            params: Some(json!({})),
        });
        let params = normalize_archive_extract_params(json!({}), &req);
        assert_eq!(params["password"], "123456");
        assert_eq!(params["format"], "自动");

        let cleaned =
            normalize_archive_extract_params(json!({"password": "password=123456.zip"}), &req);
        assert_eq!(cleaned["password"], "123456");
    }

    /// A PNG data URL is recognised as binary and offered extract/identify tools.
    #[test]
    fn binary_data_offers_analysis_tools() {
        let d = descs();
        // Minimal PNG signature encoded as a data URL.
        let png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==";
        assert!(is_binary(png));
        assert_eq!(data_kind(png), DataKind::Binary);
        let hint_ids: Vec<&str> = detect_hints(&d, DataKind::Binary, png)
            .into_iter()
            .map(|h| h.node)
            .collect();
        assert!(hint_ids.contains(&"strings"), "strings should be offered");
        assert!(
            hint_ids.contains(&"png_chunks"),
            "PNG-specific chunk tool should be offered"
        );
        // The binary catalog exposes byte/image analysis tools, not text decoders.
        let cat = build_story_catalog(&d, DataKind::Binary);
        assert!(cat.contains("strings |"));
        assert!(cat.contains("detect_file_type |"));
    }

    /// A file/image challenge can carry a typed 题干; it must reach the prompt.
    #[test]
    fn brief_reaches_the_prompt_context() {
        let req = GalgameStepRequest {
            challenge: "data:image/png;base64,iVBORw0KGgo=".into(),
            challenge_kind: "image".into(),
            brief: "附件是张 PNG，flag 藏在 LSB 里".into(),
            history: vec![],
            picked: None,
        };
        let ctx = build_context(&req, &req.challenge, None, &[]);
        assert!(ctx.contains("题干"), "context should label the brief");
        assert!(
            ctx.contains("flag 藏在 LSB 里"),
            "the brief text must be present"
        );
    }

    #[test]
    fn story_catalog_lists_decoders_and_excludes_ai() {
        let d = descs();
        let cat = build_story_catalog(&d, DataKind::Text);
        assert!(cat.contains("hex_decode |"));
        assert!(cat.contains("base64_decode |"));
        assert!(
            !cat.contains("ai_"),
            "AI/meta tools must not appear in the story catalog"
        );
    }
}
