//! Unicode escapes — `\uXXXX` / `\u{XXXX}` escape and unescape.
use super::prelude::*;

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let all = pstr(params, "mode", "仅非ASCII") == "全部";
        let mut out = String::new();
        let mut buf = [0u16; 2];
        for c in in_text(inputs, "text")?.chars() {
            if !all && c.is_ascii() {
                out.push(c);
            } else {
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
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
        let re = regex::Regex::new(r"\\u\{([0-9a-fA-F]+)\}|\\u([0-9a-fA-F]{4})").unwrap();
        let out = re.replace_all(in_text(inputs, "text")?, |caps: &regex::Captures| {
            let hex = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();
            u32::from_str_radix(hex, 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_default()
        });
        Ok(out_text(out.into_owned()))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "unicode_escape",
            ENC,
            "Unicode 转义",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select(
                "mode",
                "范围",
                &["仅非ASCII", "全部"],
                "仅非ASCII",
            )],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "unicode_unescape",
            ENC,
            "Unicode 反转义",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
