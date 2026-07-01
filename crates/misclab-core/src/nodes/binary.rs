//! Bytes ↔ binary string (8 bits per byte), configurable delimiter.
use super::basex::decoded;
use super::prelude::*;

fn sep(params: &serde_json::Value) -> &'static str {
    match pstr(params, "delimiter", "空格") {
        "无" => "",
        "逗号" => ",",
        _ => " ",
    }
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let s = data
            .iter()
            .map(|b| format!("{b:08b}"))
            .collect::<Vec<_>>()
            .join(sep(params));
        Ok(out_text(s))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let bits: Vec<u8> = in_text(inputs, "text")?
            .bytes()
            .filter(|&b| b == b'0' || b == b'1')
            .collect();
        let mut out = Vec::with_capacity(bits.len() / 8);
        for chunk in bits.chunks(8) {
            if chunk.len() < 8 {
                break;
            }
            let mut byte = 0u8;
            for &c in chunk {
                byte = (byte << 1) | (c - b'0');
            }
            out.push(byte);
        }
        Ok(decoded(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let delim = || ParamSpec::select("delimiter", "分隔符", &["空格", "无", "逗号"], "空格");
    reg.register(
        desc(
            "to_binary",
            RADIX,
            "转二进制",
            SLATE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![delim()],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "from_binary",
            RADIX,
            "二进制转文本",
            SLATE,
            vec![t_in()],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
