//! ChaCha20 / XChaCha20 stream cipher (symmetric — encrypt = decrypt).
use cipher::{KeyIvInit, StreamCipher};

use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let key = parse_bytes(pstr(params, "key", ""), pstr(params, "keyFormat", "Hex"))?;
        let nonce = parse_bytes(
            pstr(params, "nonce", ""),
            pstr(params, "nonceFormat", "Hex"),
        )?;
        let mut buf = parse_bytes(
            in_text(inputs, "text")?,
            pstr(params, "inputFormat", "UTF8"),
        )?;

        match pstr(params, "variant", "ChaCha20") {
            "XChaCha20" => {
                let mut c = chacha20::XChaCha20::new_from_slices(&key, &nonce).map_err(|_| {
                    CoreError::Parse("XChaCha20 需要 32 字节密钥、24 字节 nonce".into())
                })?;
                c.apply_keystream(&mut buf);
            }
            _ => {
                let mut c = chacha20::ChaCha20::new_from_slices(&key, &nonce).map_err(|_| {
                    CoreError::Parse("ChaCha20 需要 32 字节密钥、12 字节 nonce".into())
                })?;
                c.apply_keystream(&mut buf);
            }
        }

        let text = format_bytes(&buf, pstr(params, "outputFormat", "Hex"));
        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(text));
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(buf.into_boxed_slice())),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "chacha20",
            CRYPTO,
            "ChaCha20",
            ROSE,
            vec![req("text", "输入", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select("variant", "变体", &["ChaCha20", "XChaCha20"], "ChaCha20"),
                ParamSpec::text("key", "密钥(32字节)", "", false),
                ParamSpec::select("keyFormat", "密钥格式", &["Hex", "UTF8", "Base64"], "Hex"),
                ParamSpec::text("nonce", "Nonce", "", false),
                ParamSpec::select(
                    "nonceFormat",
                    "Nonce 格式",
                    &["Hex", "UTF8", "Base64"],
                    "Hex",
                ),
                ParamSpec::select(
                    "inputFormat",
                    "输入格式",
                    &["UTF8", "Hex", "Base64"],
                    "UTF8",
                ),
                ParamSpec::select(
                    "outputFormat",
                    "输出格式",
                    &["Hex", "Base64", "UTF8"],
                    "Hex",
                ),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
