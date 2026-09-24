//! JWT decoder — split & base64url-decode header + payload (no signature verify).
use base64::Engine as _;

use super::prelude::*;

fn b64url(part: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

fn pretty(json: &str) -> String {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
        .unwrap_or_else(|| json.to_string())
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let jwt = in_text(inputs, "text")?.trim();
        let parts: Vec<&str> = jwt.split('.').collect();
        if parts.len() < 2 {
            return Err(CoreError::Parse(
                "不是有效的 JWT（应为 header.payload.signature）".into(),
            ));
        }
        let header = pretty(&b64url(parts[0]));
        let payload = pretty(&b64url(parts[1]));
        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(payload.clone()));
        m.insert("payload".to_string(), PortValue::Text(payload));
        m.insert("header".to_string(), PortValue::Text(header));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "jwt_decode",
            UTIL,
            "JWT 解码",
            CYAN,
            vec![t_in()],
            vec![
                req("text", "载荷", PortType::Text),
                opt("payload", "payload", PortType::Text),
                opt("header", "header", PortType::Text),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
