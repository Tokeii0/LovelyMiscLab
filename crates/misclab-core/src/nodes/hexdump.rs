//! Classic xxd-style hexdump — to / from.
use super::basex::decoded;
use super::prelude::*;

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let mut out = String::new();
        for (i, chunk) in data.chunks(16).enumerate() {
            let hex = chunk
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            let ascii: String = chunk
                .iter()
                .map(|&b| {
                    if (0x20..0x7f).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            out.push_str(&format!("{:08x}  {:<47}  {}\n", i * 16, hex, ascii));
        }
        Ok(out_text(out))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut out = Vec::new();
        for line in in_text(inputs, "text")?.lines() {
            let mut toks = line.split_whitespace().peekable();
            // Skip a leading offset token (long hex, or ends with ':').
            if let Some(first) = toks.peek() {
                let f = first.trim_end_matches(':');
                if (f.len() > 2 || first.ends_with(':')) && f.chars().all(|c| c.is_ascii_hexdigit())
                {
                    toks.next();
                }
            }
            for tok in toks {
                if tok.len() == 2 && tok.chars().all(|c| c.is_ascii_hexdigit()) {
                    out.push(u8::from_str_radix(tok, 16).unwrap());
                } else {
                    break; // reached the ASCII column
                }
            }
        }
        Ok(decoded(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "to_hexdump",
            ENC,
            "转 Hexdump",
            BLUE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "from_hexdump",
            ENC,
            "Hexdump 转字节",
            BLUE,
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
