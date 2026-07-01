//! Shared single-value text transforms — the selectable "body" for the `map`
//! (for-each) and `iterate` (while) control nodes. Nodes can't invoke other nodes
//! through the registry, so a focused menu of common ops is inlined here.
use super::prelude::*;
use base64::Engine as _;
use digest::Digest;

pub const TRANSFORMS: &[&str] = &[
    "大写",
    "小写",
    "反转",
    "去空白",
    "Base64编码",
    "Base64解码",
    "Hex编码",
    "Hex解码",
    "URL编码",
    "URL解码",
    "ROT13",
    "MD5",
    "SHA1",
    "SHA256",
];

pub fn apply_transform(op: &str, s: &str) -> Result<String, CoreError> {
    Ok(match op {
        "大写" => s.to_uppercase(),
        "小写" => s.to_lowercase(),
        "反转" => s.chars().rev().collect(),
        "去空白" => s.chars().filter(|c| !c.is_whitespace()).collect(),
        "Base64编码" => base64::engine::general_purpose::STANDARD.encode(s),
        "Base64解码" => {
            let b = base64::engine::general_purpose::STANDARD
                .decode(s.trim())
                .map_err(|e| CoreError::Parse(format!("Base64: {e}")))?;
            String::from_utf8_lossy(&b).into_owned()
        }
        "Hex编码" => hex::encode(s.as_bytes()),
        "Hex解码" => {
            let b = hex::decode(s.trim().replace([' ', '\n'], ""))
                .map_err(|e| CoreError::Parse(format!("Hex: {e}")))?;
            String::from_utf8_lossy(&b).into_owned()
        }
        "URL编码" => urlencoding::encode(s).into_owned(),
        "URL解码" => urlencoding::decode(s)
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| s.to_string()),
        "ROT13" => s
            .chars()
            .map(|c| match c {
                'a'..='z' => (((c as u8 - b'a' + 13) % 26) + b'a') as char,
                'A'..='Z' => (((c as u8 - b'A' + 13) % 26) + b'A') as char,
                o => o,
            })
            .collect(),
        "MD5" => hex::encode(md5::Md5::digest(s.as_bytes())),
        "SHA1" => hex::encode(sha1::Sha1::digest(s.as_bytes())),
        "SHA256" => hex::encode(sha2::Sha256::digest(s.as_bytes())),
        other => return Err(CoreError::Parse(format!("未知操作: {other}"))),
    })
}
