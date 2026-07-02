//! Shared re-exports and helpers so each node file stays tiny.
//!
//! Convention for adding a node: create `nodes/<id>.rs`, `use super::prelude::*`,
//! implement [`Node`], build a [`NodeDescriptor`] with [`desc`], and expose
//! `pub fn register(reg: &mut NodeRegistry)`. Then add `mod <id>;` +
//! `<id>::register(reg);` in `nodes/mod.rs`. It appears in the palette
//! automatically.
#![allow(dead_code)]

pub use std::sync::Arc;

pub use crate::ai;
pub use crate::error::CoreError;
pub use crate::graph::port::{PortType, PortValue, ScoredString};
pub use crate::node::descriptor::{Cost, NodeDescriptor, ParamSpec, PortSpec};
pub use crate::node::registry::NodeRegistry;
pub use crate::node::{Node, NodeCtx, PortMap};

// Categories (palette groups).
pub const IO: &str = "输入输出";
pub const ENC: &str = "编码/加密";
pub const TXT: &str = "文本处理";
pub const CTL: &str = "控制/逻辑";
pub const ARC: &str = "压缩包";
pub const STEG: &str = "隐写术";
pub const HASH: &str = "哈希/摘要";
pub const RADIX: &str = "进制转换";
pub const CRYPTO: &str = "加密解密";
pub const CHARSET: &str = "字符编码";
pub const UTIL: &str = "工具/分析";
pub const IMG: &str = "图像处理";
pub const AI: &str = "AI";

// Node header colors.
pub const SLATE: &str = "#64748b";
pub const GREEN: &str = "#22c55e";
pub const BLUE: &str = "#3b82f6";
pub const PURPLE: &str = "#a855f7";
pub const TEAL: &str = "#14b8a6";
pub const AMBER: &str = "#f59e0b";
pub const EMERALD: &str = "#10b981";
pub const INDIGO: &str = "#6366f1";
pub const CYAN: &str = "#06b6d4";
pub const ROSE: &str = "#f43f5e";
pub const FUCHSIA: &str = "#d946ef";

/// Read a required Text input by port name.
pub fn in_text<'a>(inputs: &'a PortMap, name: &str) -> Result<&'a str, CoreError> {
    inputs
        .get(name)
        .ok_or_else(|| CoreError::MissingInput(name.to_string()))?
        .as_text()
}

/// Read an input as raw bytes: Bytes as-is, Text as its UTF-8 bytes.
pub fn in_bytes(inputs: &PortMap, name: &str) -> Result<Vec<u8>, CoreError> {
    match inputs.get(name) {
        Some(PortValue::Bytes(b)) => Ok(b.to_vec()),
        Some(PortValue::Text(s)) => Ok(s.as_bytes().to_vec()),
        Some(PortValue::None) | None => Err(CoreError::MissingInput(name.to_string())),
        Some(other) => Err(CoreError::Type(format!(
            "expected Bytes or Text, got {:?}",
            other.port_type()
        ))),
    }
}

/// Parse a key/IV string given a format: `UTF8` (raw bytes), `Hex`, or `Base64`.
pub fn parse_bytes(text: &str, format: &str) -> Result<Vec<u8>, CoreError> {
    match format {
        "Hex" => hex::decode(text.trim().replace([' ', '\n'], ""))
            .map_err(|e| CoreError::Parse(format!("Hex 无效: {e}"))),
        "Base64" => {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD
                .decode(text.trim())
                .map_err(|e| CoreError::Parse(format!("Base64 无效: {e}")))
        }
        _ => Ok(text.as_bytes().to_vec()),
    }
}

/// Render bytes as a string in the given format: `Hex`, `Base64`, or `UTF8` (lossy).
pub fn format_bytes(data: &[u8], format: &str) -> String {
    match format {
        "Hex" => hex::encode(data),
        "Base64" => {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD.encode(data)
        }
        _ => String::from_utf8_lossy(data).into_owned(),
    }
}

/// A single-entry output map.
pub fn one(name: &str, value: PortValue) -> PortMap {
    let mut m = PortMap::new();
    m.insert(name.to_string(), value);
    m
}

/// Shortcut for a single Text output on port "text".
pub fn out_text(value: String) -> PortMap {
    one("text", PortValue::Text(value))
}

pub fn pstr<'a>(p: &'a serde_json::Value, name: &str, default: &'a str) -> &'a str {
    p.get(name).and_then(|v| v.as_str()).unwrap_or(default)
}

pub fn pbool(p: &serde_json::Value, name: &str, default: bool) -> bool {
    p.get(name).and_then(|v| v.as_bool()).unwrap_or(default)
}

pub fn pnum(p: &serde_json::Value, name: &str, default: f64) -> f64 {
    p.get(name).and_then(|v| v.as_f64()).unwrap_or(default)
}

/// Read an input as a list of strings: StringList / Candidates (their text) /
/// Text (split into lines). The currency of the list-processing control nodes.
pub fn in_list(inputs: &PortMap, name: &str) -> Result<Vec<String>, CoreError> {
    match inputs.get(name) {
        Some(PortValue::StringList(v)) => Ok(v.clone()),
        Some(PortValue::Candidates(c)) => Ok(c.iter().map(|s| s.text.clone()).collect()),
        Some(PortValue::Text(t)) => Ok(t.lines().map(|l| l.to_string()).collect()),
        Some(PortValue::None) | None => Err(CoreError::MissingInput(name.to_string())),
        Some(other) => Err(CoreError::Type(format!(
            "expected StringList, got {:?}",
            other.port_type()
        ))),
    }
}

pub fn desc(
    id: &str,
    category: &str,
    name: &str,
    color: &str,
    inputs: Vec<PortSpec>,
    outputs: Vec<PortSpec>,
    params: Vec<ParamSpec>,
) -> NodeDescriptor {
    NodeDescriptor {
        id: id.to_string(),
        category: category.to_string(),
        display_name: name.to_string(),
        description: String::new(),
        color: color.to_string(),
        inputs,
        outputs,
        params,
        cost: Cost::Cheap,
    }
}

pub fn req(name: &str, label: &str, ty: PortType) -> PortSpec {
    PortSpec::new(name, label, ty, true)
}

pub fn opt(name: &str, label: &str, ty: PortType) -> PortSpec {
    PortSpec::new(name, label, ty, false)
}

/// The common single Text input / single Text output port pair.
pub fn t_in() -> PortSpec {
    req("text", "输入", PortType::Text)
}
pub fn t_out() -> PortSpec {
    req("text", "输出", PortType::Text)
}

/// Heuristic "looks like meaningful text" score (favors letters/spaces, penalizes
/// control chars, bonus for `{}`). Used to rank brute-force candidates.
pub fn english_score(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0f32;
    let mut n = 0.0f32;
    for c in text.chars() {
        n += 1.0;
        sum += if c.is_ascii_alphabetic() || c == ' ' {
            1.0
        } else if c.is_ascii_digit() {
            0.6
        } else if matches!(c, '\n' | '\t' | '\r') {
            0.4
        } else if c.is_ascii_punctuation() {
            0.3
        } else if c.is_control() {
            -3.0
        } else {
            0.1
        };
    }
    let mut score = sum / n;
    if text.contains('{') && text.contains('}') {
        score += 0.5;
    }
    score
}

/// Shared param list for base64 nodes (variable code table).
pub fn base64_params() -> Vec<ParamSpec> {
    vec![
        ParamSpec::select("variant", "码表", &["标准", "URL安全", "自定义"], "标准"),
        ParamSpec::text("alphabet", "自定义码表(64字符)", "", false),
        ParamSpec::toggle("padding", "填充", true),
    ]
}

/// Build a base64 engine honoring the node's code-table params.
pub fn base64_engine(
    p: &serde_json::Value,
) -> Result<base64::engine::GeneralPurpose, CoreError> {
    let alphabet = match pstr(p, "variant", "标准") {
        "URL安全" => base64::alphabet::URL_SAFE,
        "自定义" => base64::alphabet::Alphabet::new(pstr(p, "alphabet", ""))
            .map_err(|e| CoreError::Parse(format!("自定义码表无效: {e}")))?,
        _ => base64::alphabet::STANDARD,
    };
    let config = base64::engine::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent)
        .with_encode_padding(pbool(p, "padding", true));
    Ok(base64::engine::GeneralPurpose::new(&alphabet, config))
}
