//! AI-friendly conversion between JSON and [`PortValue`].
//!
//! Two `PortValue` variants would flood the AI's context if passed raw: `Bytes`
//! (serializes as a JSON number array) and `Image` (a huge base64 data-url). So
//! on the way **in** we accept bytes/images as base64 or file paths, and on the
//! way **out** we spill them to files and return a path + metadata. Long text is
//! truncated; candidate lists are capped.

use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::{json, Value};

use misclab_core::graph::port::PortValue;

use crate::mcp::state::McpState;

const TEXT_LIMIT: usize = 8192;
const CAND_LIMIT: usize = 20;
const B64: base64::engine::general_purpose::GeneralPurpose = base64::engine::general_purpose::STANDARD;

// ---------------------------------------------------------------------------
// JSON -> PortValue
// ---------------------------------------------------------------------------

/// Convert AI-supplied JSON into a [`PortValue`]. Accepts bare strings/numbers/
/// bools as Text/Number/Bool, `{type,value}` for the standard variants, and for
/// `bytes`/`image` also `{base64}` / `{path}` / data-url convenience forms.
pub fn port_value_in(v: &Value, state: &McpState) -> Result<PortValue, String> {
    match v {
        Value::String(s) => Ok(PortValue::Text(s.clone())),
        Value::Number(n) => Ok(PortValue::Number(n.as_f64().unwrap_or(0.0))),
        Value::Bool(b) => Ok(PortValue::Bool(*b)),
        Value::Null => Ok(PortValue::None),
        Value::Object(map) => match map.get("type").and_then(|t| t.as_str()) {
            Some("bytes") => Ok(PortValue::Bytes(decode_bytes(map.get("value"), state)?.into())),
            Some("image") => Ok(PortValue::Image(decode_image(map.get("value"), state)?)),
            Some(_) => serde_json::from_value(v.clone()).map_err(|e| format!("invalid port value: {e}")),
            None => Ok(PortValue::Json(v.clone())),
        },
        // arrays and anything else: treat as opaque JSON
        _ => Ok(PortValue::Json(v.clone())),
    }
}

fn decode_bytes(v: Option<&Value>, state: &McpState) -> Result<Vec<u8>, String> {
    let v = v.ok_or("bytes: missing value")?;
    if let Some(s) = v.as_str() {
        return b64_decode(s);
    }
    if let Some(b64) = v.get("base64").and_then(|x| x.as_str()) {
        return b64_decode(b64);
    }
    if let Some(path) = v.get("path").and_then(|x| x.as_str()) {
        return read_guarded(path, state);
    }
    Err("bytes: value must be a base64 string, {base64:...}, or {path:...}".into())
}

fn decode_image(v: Option<&Value>, state: &McpState) -> Result<String, String> {
    let v = v.ok_or("image: missing value")?;
    if let Some(s) = v.as_str() {
        if s.starts_with("data:") {
            return Ok(s.to_string());
        }
        return image_from_path(s, state);
    }
    if let Some(p) = v.get("path").and_then(|x| x.as_str()) {
        return image_from_path(p, state);
    }
    if let Some(u) = v.get("dataUrl").and_then(|x| x.as_str()) {
        return Ok(u.to_string());
    }
    Err("image: value must be a data-url, a path string, or {path:...}".into())
}

fn image_from_path(path: &str, state: &McpState) -> Result<String, String> {
    let bytes = read_guarded(path, state)?;
    Ok(format!("data:{};base64,{}", mime_from_ext(path), B64.encode(&bytes)))
}

fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    B64.decode(s.trim()).map_err(|e| format!("base64 decode failed: {e}"))
}

/// Read a file for an input. Minimal guard: it must exist and be a regular file.
/// (The endpoint is bearer-gated and already a code-exec surface, so this is a
/// sanity check, not a sandbox.)
fn read_guarded(path: &str, _state: &McpState) -> Result<Vec<u8>, String> {
    let p = Path::new(path);
    if !p.is_file() {
        return Err(format!("no such file: {path}"));
    }
    std::fs::read(p).map_err(|e| format!("read {path} failed: {e}"))
}

// ---------------------------------------------------------------------------
// PortValue -> JSON
// ---------------------------------------------------------------------------

/// Convert a [`PortValue`] into AI-friendly JSON: bytes/images spilled to files,
/// long text truncated, candidate lists capped.
pub fn port_value_out(v: &PortValue, state: &McpState) -> Value {
    match v {
        PortValue::Text(s) if s.chars().count() > TEXT_LIMIT => {
            let head: String = s.chars().take(TEXT_LIMIT).collect();
            json!({ "type": "text", "value": head, "truncated": true, "fullLen": s.chars().count() })
        }
        PortValue::Bytes(b) => bytes_out(b, state),
        PortValue::Image(url) => image_out(url, state),
        PortValue::Candidates(c) if c.len() > CAND_LIMIT => {
            json!({ "type": "candidates", "value": &c[..CAND_LIMIT], "truncated": true, "total": c.len() })
        }
        other => serde_json::to_value(other).unwrap_or(Value::Null),
    }
}

fn bytes_out(b: &[u8], state: &McpState) -> Value {
    let dir = out_dir(state);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.bin", uuid::Uuid::new_v4().simple()));
    let saved = std::fs::write(&path, b).is_ok();
    let preview: String = b.iter().take(64).map(|x| format!("{x:02x}")).collect();
    json!({
        "type": "bytes",
        "value": {
            "path": if saved { path.to_string_lossy().to_string() } else { String::new() },
            "len": b.len(),
            "previewHex": preview,
        }
    })
}

fn image_out(url: &str, state: &McpState) -> Value {
    let (mime, bytes) = decode_data_url(url);
    let dir = out_dir(state);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.{}", uuid::Uuid::new_v4().simple(), ext_from_mime(&mime)));
    let (saved, len) = match &bytes {
        Some(b) => (std::fs::write(&path, b).is_ok(), b.len()),
        None => (false, 0),
    };
    json!({
        "type": "image",
        "value": {
            "path": if saved { path.to_string_lossy().to_string() } else { String::new() },
            "mime": mime,
            "len": len,
        }
    })
}

/// `<settings.output_dir>/mcp` if set, else `<app_data_dir>/mcp-out`.
fn out_dir(state: &McpState) -> PathBuf {
    let out = state.settings.lock().expect("settings mutex poisoned").output_dir.clone();
    if !out.trim().is_empty() {
        return Path::new(&out).join("mcp");
    }
    state
        .app
        .app_data_dir()
        .map(|d| d.join("mcp-out"))
        .unwrap_or_else(|| std::env::temp_dir().join("misclab-mcp-out"))
}

fn decode_data_url(url: &str) -> (String, Option<Vec<u8>>) {
    if let Some(rest) = url.strip_prefix("data:") {
        if let Some((meta, data)) = rest.split_once(',') {
            let mime = meta.split(';').next().unwrap_or("application/octet-stream").to_string();
            let bytes = if meta.contains("base64") {
                B64.decode(data).ok()
            } else {
                Some(data.as_bytes().to_vec())
            };
            return (mime, bytes);
        }
    }
    ("application/octet-stream".to_string(), None)
}

fn ext_from_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/webp" => "webp",
        _ => "bin",
    }
}

fn mime_from_ext(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_url_round_trips() {
        let raw = b"\x89PNG\r\n\x1a\n hello";
        let url = format!("data:image/png;base64,{}", B64.encode(raw));
        let (mime, bytes) = decode_data_url(&url);
        assert_eq!(mime, "image/png");
        assert_eq!(bytes.as_deref(), Some(&raw[..]));
        assert_eq!(ext_from_mime(&mime), "png");
    }

    #[test]
    fn non_data_url_is_rejected() {
        let (mime, bytes) = decode_data_url("not a data url");
        assert_eq!(mime, "application/octet-stream");
        assert!(bytes.is_none());
    }

    #[test]
    fn mime_ext_mapping() {
        assert_eq!(mime_from_ext("a/b/c.JPG"), "image/jpeg");
        assert_eq!(mime_from_ext("x.webp"), "image/webp");
        assert_eq!(mime_from_ext("x.unknown"), "application/octet-stream");
        assert_eq!(ext_from_mime("image/gif"), "gif");
    }

    #[test]
    fn b64_decode_trims_and_decodes() {
        assert_eq!(b64_decode("  aGk=  ").unwrap(), b"hi");
        assert!(b64_decode("!!!not-base64!!!").is_err());
    }
}
