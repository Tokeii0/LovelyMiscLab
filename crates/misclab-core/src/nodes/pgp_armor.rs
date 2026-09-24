//! PGP ASCII-Armor (RFC 4880 §6): unwrap an armored block to its binary
//! packets (with CRC-24 check), or wrap binary packets back into armor.
use base64::Engine as _;

use super::prelude::*;

const CRC24_INIT: u32 = 0x00B7_04CE;
const CRC24_POLY: u32 = 0x0186_4CFB;

fn crc24(data: &[u8]) -> u32 {
    let mut crc = CRC24_INIT;
    for &b in data {
        crc ^= (b as u32) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x0100_0000 != 0 {
                crc ^= CRC24_POLY;
            }
        }
    }
    crc & 0x00FF_FFFF
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

struct Dearmor;
impl Node for Dearmor {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let mut block_type = String::new();
        let mut body = String::new();
        let mut crc_line: Option<String> = None;
        let mut in_body = false;
        let mut past_headers = false;
        for line in text.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("-----BEGIN ") {
                block_type = rest.trim_end_matches('-').trim().to_string();
                in_body = true;
                past_headers = false;
                continue;
            }
            if t.starts_with("-----END ") {
                break;
            }
            if !in_body {
                continue;
            }
            if !past_headers {
                // Armor headers (Version:, Comment:…) end at the first blank line.
                if t.is_empty() {
                    past_headers = true;
                    continue;
                }
                if t.contains(':') {
                    continue;
                }
                past_headers = true; // no headers present
            }
            if let Some(crc) = t.strip_prefix('=') {
                crc_line = Some(crc.to_string());
            } else if !t.is_empty() {
                body.push_str(t);
            }
        }
        if block_type.is_empty() {
            return Err(CoreError::Parse(
                "未找到 PGP 装甲块（-----BEGIN …-----）".into(),
            ));
        }
        let bytes = b64()
            .decode(body.replace(char::is_whitespace, ""))
            .map_err(|e| CoreError::Parse(format!("Base64 无效: {e}")))?;
        let crc_ok = match crc_line {
            Some(c) => {
                let want = b64()
                    .decode(c.trim())
                    .map_err(|e| CoreError::Parse(format!("CRC Base64 无效: {e}")))?;
                want.len() == 3
                    && ((want[0] as u32) << 16 | (want[1] as u32) << 8 | want[2] as u32)
                        == crc24(&bytes)
            }
            None => true,
        };
        let mut m = PortMap::new();
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(bytes.clone().into_boxed_slice())),
        );
        m.insert("hex".to_string(), PortValue::Text(hex::encode(&bytes)));
        m.insert("type".to_string(), PortValue::Text(block_type));
        m.insert("crcOk".to_string(), PortValue::Bool(crc_ok));
        Ok(m)
    }
}

struct Enarmor;
impl Node for Enarmor {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let bt = pstr(p, "blockType", "MESSAGE");
        let encoded = b64().encode(&data);
        let mut out = format!("-----BEGIN PGP {bt}-----\n\n");
        for chunk in encoded.as_bytes().chunks(64) {
            out.push_str(std::str::from_utf8(chunk).unwrap());
            out.push('\n');
        }
        let crc = crc24(&data);
        let crc_bytes = [(crc >> 16) as u8, (crc >> 8) as u8, crc as u8];
        out.push('=');
        out.push_str(&b64().encode(crc_bytes));
        out.push('\n');
        out.push_str(&format!("-----END PGP {bt}-----\n"));
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "pgp_dearmor",
            CRYPTO,
            "PGP 解甲(Dearmor)",
            ROSE,
            vec![t_in()],
            vec![
                req("bytes", "字节", PortType::Bytes),
                opt("hex", "hex", PortType::Text),
                opt("type", "块类型", PortType::Text),
                opt("crcOk", "CRC 校验", PortType::Bool),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(Dearmor)),
    );
    reg.register(
        desc(
            "pgp_enarmor",
            CRYPTO,
            "PGP 装甲(Enarmor)",
            ROSE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![ParamSpec::select(
                "blockType",
                "块类型",
                &[
                    "MESSAGE",
                    "PUBLIC KEY BLOCK",
                    "PRIVATE KEY BLOCK",
                    "SIGNATURE",
                ],
                "MESSAGE",
            )],
        ),
        Arc::new(|| Arc::new(Enarmor)),
    );
}
