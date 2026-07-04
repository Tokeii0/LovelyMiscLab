//! Quoted-Printable (RFC 2045) encode/decode — common in email forensics.
use super::prelude::*;

fn emit(out: &mut String, chunk: &str, line: &mut usize) {
    if *line + chunk.len() > 75 {
        out.push_str("=\r\n");
        *line = 0;
    }
    out.push_str(chunk);
    *line += chunk.len();
}

fn qp_encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut line = 0usize;
    for (i, &b) in data.iter().enumerate() {
        if b == b'\n' {
            out.push_str("\r\n");
            line = 0;
            continue;
        }
        if b == b'\r' {
            continue;
        }
        let is_ws = b == b' ' || b == b'\t';
        let next_break = matches!(data.get(i + 1), Some(b'\r') | Some(b'\n') | None);
        if ((33..=126).contains(&b) && b != b'=') || (is_ws && !next_break) {
            emit(&mut out, &(b as char).to_string(), &mut line);
        } else {
            emit(&mut out, &format!("={b:02X}"), &mut line);
        }
    }
    out
}

fn qp_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' {
            if i + 1 < b.len() && b[i + 1] == b'\n' {
                i += 2;
                continue;
            }
            if i + 2 < b.len() && b[i + 1] == b'\r' && b[i + 2] == b'\n' {
                i += 3;
                continue;
            }
            if i + 2 < b.len() {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
            }
            out.push(b'=');
            i += 1;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

struct Enc;
impl Node for Enc {
    fn run(&self, inputs: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        Ok(out_text(qp_encode(&in_bytes(inputs, "data")?)))
    }
}

struct Dec;
impl Node for Dec {
    fn run(&self, inputs: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let bytes = qp_decode(in_text(inputs, "text")?);
        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(String::from_utf8_lossy(&bytes).into_owned()));
        m.insert("bytes".to_string(), PortValue::Bytes(Arc::from(bytes.into_boxed_slice())));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "quoted_printable_encode",
            CHARSET,
            "Quoted-Printable 编码",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "quoted_printable_decode",
            CHARSET,
            "Quoted-Printable 解码",
            INDIGO,
            vec![t_in()],
            vec![req("text", "文本", PortType::Text), opt("bytes", "字节", PortType::Bytes)],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
