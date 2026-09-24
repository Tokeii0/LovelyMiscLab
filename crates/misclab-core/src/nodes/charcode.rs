//! To/From Charcode — render each character as its numeric code point in a
//! chosen base, and back. Mirrors CyberChef's "To Charcode" / "From Charcode".
use super::prelude::*;

fn radix(p: &serde_json::Value) -> u32 {
    match pstr(p, "base", "16") {
        "10" => 10,
        "8" => 8,
        "2" => 2,
        _ => 16,
    }
}

fn fmt(n: u32, base: u32) -> String {
    match base {
        10 => n.to_string(),
        8 => format!("{n:o}"),
        2 => format!("{n:b}"),
        _ => format!("{n:x}"),
    }
}

fn delim(p: &serde_json::Value) -> &'static str {
    match pstr(p, "delimiter", "空格") {
        "逗号" => ", ",
        "换行" => "\n",
        "分号" => "; ",
        _ => " ",
    }
}

struct ToCharcode;
impl Node for ToCharcode {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let base = radix(p);
        let parts: Vec<String> = text.chars().map(|c| fmt(c as u32, base)).collect();
        Ok(out_text(parts.join(delim(p))))
    }
}

struct FromCharcode;
impl Node for FromCharcode {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let base = radix(p);
        let mut s = String::new();
        for tok in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
            let tok = tok.trim();
            if tok.is_empty() {
                continue;
            }
            let n = u32::from_str_radix(tok, base)
                .map_err(|_| CoreError::Parse(format!("无法按 {base} 进制解析: {tok}")))?;
            match char::from_u32(n) {
                Some(ch) => s.push(ch),
                None => return Err(CoreError::Parse(format!("非法码点: {n}"))),
            }
        }
        Ok(out_text(s))
    }
}

fn params() -> Vec<ParamSpec> {
    vec![
        ParamSpec::select("base", "进制", &["16", "10", "8", "2"], "16"),
        ParamSpec::select(
            "delimiter",
            "分隔符",
            &["空格", "逗号", "换行", "分号"],
            "空格",
        ),
    ]
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "to_charcode",
            RADIX,
            "字符转码点",
            INDIGO,
            vec![t_in()],
            vec![t_out()],
            params(),
        ),
        Arc::new(|| Arc::new(ToCharcode)),
    );
    reg.register(
        desc(
            "from_charcode",
            RADIX,
            "码点转字符",
            INDIGO,
            vec![t_in()],
            vec![t_out()],
            params(),
        ),
        Arc::new(|| Arc::new(FromCharcode)),
    );
}
