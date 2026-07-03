//! JWT 密钥爆破：对 HS256/384/512 签名的 JWT，用字典逐个试密钥，
//! 重算 `HMAC(header.payload)` 与签名段比对，命中即得密钥。
//! 复用 `jwt` 的 base64url、`hash` 的 HMAC 路径，循环结构同 `hash_crack`。
use base64::Engine as _;
use hmac::{Hmac, Mac};

use super::prelude::*;

fn b64url_decode(part: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .ok()
}

/// 用给定算法与密钥对签名输入算 HMAC，返回 base64url（无填充）签名串。
fn sign(alg: &str, key: &[u8], msg: &[u8]) -> Option<String> {
    let raw: Vec<u8> = match alg {
        "HS256" => {
            let mut m = Hmac::<sha2::Sha256>::new_from_slice(key).ok()?;
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
        "HS384" => {
            let mut m = Hmac::<sha2::Sha384>::new_from_slice(key).ok()?;
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
        "HS512" => {
            let mut m = Hmac::<sha2::Sha512>::new_from_slice(key).ok()?;
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
        _ => return None,
    };
    Some(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw))
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let jwt = in_text(i, "text")?.trim();
        let parts: Vec<&str> = jwt.split('.').collect();
        if parts.len() != 3 {
            return Err(CoreError::Parse(
                "不是有效的 JWT（应为 header.payload.signature）。".into(),
            ));
        }
        // 确定算法：参数指定优先，否则从 header 的 alg 读取。
        let mut alg = pstr(p, "algorithm", "自动").to_string();
        if alg == "自动" {
            alg = b64url_decode(parts[0])
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| v.get("alg").and_then(|a| a.as_str()).map(str::to_string))
                .unwrap_or_else(|| "HS256".into());
        }
        if !matches!(alg.as_str(), "HS256" | "HS384" | "HS512") {
            return Err(CoreError::Parse(format!(
                "只支持 HMAC 系列（HS256/384/512），该 JWT 的 alg = {alg}。"
            )));
        }
        let words = in_list(i, "wordlist")?;
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let target = parts[2];

        let mut hit: Option<String> = None;
        for (n, w) in words.iter().enumerate() {
            if n % 2000 == 0 {
                ctx.check_cancel()?;
            }
            if sign(&alg, w.as_bytes(), signing_input.as_bytes()).as_deref() == Some(target) {
                hit = Some(w.clone());
                break;
            }
        }

        let mut m = PortMap::new();
        match &hit {
            Some(k) => {
                m.insert("text".into(), PortValue::Text(k.clone()));
                m.insert("found".into(), PortValue::Bool(true));
                m.insert(
                    "report".into(),
                    PortValue::Text(format!(
                        "命中！{alg} 密钥 = \"{k}\"（试了 {} 个候选）。",
                        words.len()
                    )),
                );
            }
            None => {
                m.insert("text".into(), PortValue::Text(String::new()));
                m.insert("found".into(), PortValue::Bool(false));
                m.insert(
                    "report".into(),
                    PortValue::Text(format!(
                        "字典 {} 个候选均未命中（{alg}）。扩充字典再试。",
                        words.len()
                    )),
                );
            }
        }
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "jwt_crack",
            UTIL,
            "JWT 密钥爆破",
            CYAN,
            vec![
                req("text", "JWT", PortType::Text),
                req("wordlist", "字典", PortType::Any),
            ],
            vec![
                req("text", "密钥", PortType::Text),
                opt("found", "命中", PortType::Bool),
                opt("report", "信息", PortType::Text),
            ],
            vec![ParamSpec::select(
                "algorithm",
                "算法",
                &["自动", "HS256", "HS384", "HS512"],
                "自动",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    fn make_jwt(secret: &[u8]) -> String {
        let enc = |b: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b);
        let h = enc(br#"{"alg":"HS256","typ":"JWT"}"#);
        let pl = enc(br#"{"user":"admin"}"#);
        let signing = format!("{h}.{pl}");
        let sig = sign("HS256", secret, signing.as_bytes()).unwrap();
        format!("{signing}.{sig}")
    }

    #[test]
    fn cracks_weak_secret() {
        let jwt = make_jwt(b"secret");
        let mut inputs = PortMap::new();
        inputs.insert("text".into(), PortValue::Text(jwt));
        inputs.insert(
            "wordlist".into(),
            PortValue::StringList(vec![
                "admin".into(),
                "password".into(),
                "secret".into(),
                "123456".into(),
            ]),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "jwt_crack",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("found"), Some(PortValue::Bool(true))));
        assert_eq!(
            match out.get("text") {
                Some(PortValue::Text(s)) => s.clone(),
                o => panic!("{o:?}"),
            },
            "secret"
        );
    }

    #[test]
    fn reports_miss() {
        let jwt = make_jwt(b"correct-horse");
        let mut inputs = PortMap::new();
        inputs.insert("text".into(), PortValue::Text(jwt));
        inputs.insert(
            "wordlist".into(),
            PortValue::StringList(vec!["a".into(), "b".into()]),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "jwt_crack",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("found"), Some(PortValue::Bool(false))));
    }
}
