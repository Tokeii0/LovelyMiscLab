use super::prelude::*;
use base64::Engine as _;

/// Apply one decode step of the chosen codec, or None if it can't be decoded.
fn decode_once(codec: &str, s: &str) -> Option<String> {
    let cleaned: String = s.split_whitespace().collect();
    match codec {
        "Base64" => base64::engine::general_purpose::STANDARD
            .decode(cleaned.as_bytes())
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned()),
        "Hex" => hex::decode(&cleaned)
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned()),
        "URL" => urlencoding::decode(s).ok().map(|c| c.into_owned()),
        _ => None,
    }
}

/// Repeatedly decode until an exit condition — the loop the DAG itself can't do.
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut current = in_text(inputs, "text")?.to_string();
        let codec = pstr(params, "codec", "Base64");
        let until = pstr(params, "until", "无法继续");
        let pattern = pstr(params, "pattern", r"flag\{[^}]*\}");
        let max = params.get("max").and_then(|v| v.as_f64()).unwrap_or(16.0) as usize;
        let re = regex::Regex::new(pattern).ok();

        let mut iterations = 0usize;
        let mut hit = false;
        for _ in 0..max.max(1) {
            if until == "匹配正则" {
                if let Some(re) = &re {
                    if re.is_match(&current) {
                        hit = true;
                        break;
                    }
                }
            }
            match decode_once(codec, &current) {
                Some(next) if next != current => {
                    current = next;
                    iterations += 1;
                }
                _ => break, // can't decode further / no change
            }
        }
        if until == "匹配正则" {
            if let Some(re) = &re {
                hit = re.is_match(&current);
            }
        }

        let mut out = PortMap::new();
        out.insert("text".to_string(), PortValue::Text(current));
        out.insert("iterations".to_string(), PortValue::Number(iterations as f64));
        out.insert("hit".to_string(), PortValue::Bool(hit));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "loop_decode",
            ENC,
            "循环解码",
            PURPLE,
            vec![t_in()],
            vec![
                req("text", "结果", PortType::Text),
                opt("iterations", "次数", PortType::Number),
                opt("hit", "命中", PortType::Bool),
            ],
            vec![
                ParamSpec::select("codec", "编码", &["Base64", "Hex", "URL"], "Base64"),
                ParamSpec::select("until", "退出条件", &["无法继续", "匹配正则"], "无法继续"),
                ParamSpec::text("pattern", "正则(匹配正则时)", r"flag\{[^}]*\}", false),
                ParamSpec::number("max", "最大次数", 1.0, 100.0, 1.0, 16.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
