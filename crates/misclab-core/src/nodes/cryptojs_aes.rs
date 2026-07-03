//! CryptoJS AES 解密：`CryptoJS.AES.encrypt(pt, passphrase)` 产出的 base64，其字节以
//! ASCII "Salted__"(8)+salt(8)+密文 开头（与 OpenSSL `enc -aes-*-cbc -md md5` 同格式）。
//! 密钥/IV 由 EVP_BytesToKey(MD5、0 迭代) 派生，默认 AES-256-CBC + PKCS7。可给字典逐口令爆破。
use base64::Engine as _;
use digest::Digest;
use md5::Md5;

use super::aes::cbc;
use super::prelude::*;

/// OpenSSL EVP_BytesToKey（MD5、无迭代）派生 key‖iv，返回 (key, iv[16])。
fn evp_bytes_to_key(pass: &[u8], salt: &[u8], key_len: usize) -> (Vec<u8>, Vec<u8>) {
    let total = key_len + 16;
    let mut out: Vec<u8> = Vec::with_capacity(total);
    let mut prev: Vec<u8> = Vec::new();
    while out.len() < total {
        let mut h = Md5::new();
        h.update(&prev);
        h.update(pass);
        h.update(salt);
        prev = h.finalize().to_vec();
        out.extend_from_slice(&prev);
    }
    (out[..key_len].to_vec(), out[key_len..total].to_vec())
}

/// 解析 "Salted__" base64 → (salt, ciphertext)。
fn parse_salted(b64: &str) -> Result<(Vec<u8>, Vec<u8>), CoreError> {
    let cleaned: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|e| CoreError::Parse(format!("Base64 无效: {e}")))?;
    if raw.len() < 16 || &raw[..8] != b"Salted__" {
        return Err(CoreError::Parse(
            "不是 CryptoJS 加盐格式（应为 base64，明文头 \"Salted__\"）。".into(),
        ));
    }
    Ok((raw[8..16].to_vec(), raw[16..].to_vec()))
}

fn key_len_of(p: &serde_json::Value) -> usize {
    match pstr(p, "keySize", "256") {
        "128" => 16,
        "192" => 24,
        _ => 32,
    }
}

fn decrypt(salt: &[u8], ct: &[u8], pass: &[u8], key_len: usize) -> Option<Vec<u8>> {
    let (key, iv) = evp_bytes_to_key(pass, salt, key_len);
    cbc(false, &key, &iv, ct).ok()
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let (salt, ct) = parse_salted(in_text(i, "text")?)?;
        let key_len = key_len_of(p);
        let out_fmt = pstr(p, "outputFormat", "UTF8");

        let words = match i.get("wordlist") {
            Some(PortValue::None) | None => Vec::new(),
            Some(_) => in_list(i, "wordlist").unwrap_or_default(),
        };

        let mut m = PortMap::new();
        if !words.is_empty() {
            for (n, w) in words.iter().enumerate() {
                if n % 500 == 0 {
                    ctx.check_cancel()?;
                }
                if let Some(pt) = decrypt(&salt, &ct, w.as_bytes(), key_len) {
                    // 命中判据：PKCS7 通过且解出有效 UTF-8（错误口令几乎必失败）。
                    if !pt.is_empty() && std::str::from_utf8(&pt).is_ok() {
                        m.insert("text".into(), PortValue::Text(format_bytes(&pt, out_fmt)));
                        m.insert(
                            "bytes".into(),
                            PortValue::Bytes(Arc::from(pt.into_boxed_slice())),
                        );
                        m.insert(
                            "report".into(),
                            PortValue::Text(format!("命中口令 \"{w}\"（试了 {} 个）。", n + 1)),
                        );
                        return Ok(m);
                    }
                }
            }
            return Err(CoreError::Other(format!(
                "字典 {} 个口令均未解出有效明文。",
                words.len()
            )));
        }

        let pass = pstr(p, "password", "");
        let pt = decrypt(&salt, &ct, pass.as_bytes(), key_len)
            .ok_or_else(|| CoreError::Other("解密失败：口令错误，或密文/填充无效。".into()))?;
        m.insert("text".into(), PortValue::Text(format_bytes(&pt, out_fmt)));
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(pt.into_boxed_slice())),
        );
        m.insert("report".into(), PortValue::Text("解密完成。".into()));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "cryptojs_aes",
            CRYPTO,
            "CryptoJS AES 解密",
            ROSE,
            vec![
                req("text", "密文(base64)", PortType::Text),
                opt("wordlist", "字典(可选爆破)", PortType::Any),
            ],
            vec![
                req("text", "明文", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
                opt("report", "信息", PortType::Text),
            ],
            vec![
                ParamSpec::text("password", "口令", "", false),
                ParamSpec::select("keySize", "密钥长度", &["128", "192", "256"], "256"),
                ParamSpec::select("outputFormat", "输出格式", &["UTF8", "Hex", "Base64"], "UTF8"),
            ],
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

    // 由 `openssl enc -aes-256-cbc -md md5 -pass pass:s3cr3t` 产出（CryptoJS 同格式），
    // 明文 "flag{cryptojs_aes}"。
    const GOLDEN: &str = "U2FsdGVkX18ZOnnUZwj6rGEuoNQ5vpeCp52RiR3b7UvRqp5miymFVykwHvZSdJWW";

    fn text_of(out: &PortMap) -> String {
        match out.get("text") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn decrypts_openssl_golden() {
        let mut inputs = PortMap::new();
        inputs.insert("text".into(), PortValue::Text(GOLDEN.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "cryptojs_aes",
            &inputs,
            &serde_json::json!({"password":"s3cr3t"}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(text_of(&out), "flag{cryptojs_aes}");
    }

    #[test]
    fn cracks_with_wordlist() {
        let mut inputs = PortMap::new();
        inputs.insert("text".into(), PortValue::Text(GOLDEN.into()));
        inputs.insert(
            "wordlist".into(),
            PortValue::StringList(vec!["a".into(), "b".into(), "s3cr3t".into()]),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "cryptojs_aes",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(text_of(&out), "flag{cryptojs_aes}");
    }

    #[test]
    fn wrong_password_does_not_yield_flag() {
        let mut inputs = PortMap::new();
        inputs.insert("text".into(), PortValue::Text(GOLDEN.into()));
        let res = GraphExecutor::run_node(
            &default_registry(),
            "cryptojs_aes",
            &inputs,
            &serde_json::json!({"password":"nope"}),
            &NullSink,
            &CancellationToken::new(),
        );
        let got = res
            .map(|o| match o.get("text") {
                Some(PortValue::Text(s)) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();
        assert_ne!(got, "flag{cryptojs_aes}");
    }
}
