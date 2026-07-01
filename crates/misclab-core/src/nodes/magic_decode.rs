use std::collections::{HashSet, VecDeque};

use super::prelude::*;
use base64::Engine as _;

/// All one-step decodings of `s` as (codec-name, result).
fn expand(s: &str) -> Vec<(&'static str, String)> {
    let cleaned: String = s.split_whitespace().collect();
    let mut out = Vec::new();
    if let Ok(b) = base64::engine::general_purpose::STANDARD.decode(cleaned.as_bytes()) {
        let t = String::from_utf8_lossy(&b).into_owned();
        if t != s && !t.is_empty() {
            out.push(("base64", t));
        }
    }
    if let Ok(b) = hex::decode(&cleaned) {
        let t = String::from_utf8_lossy(&b).into_owned();
        if t != s && !t.is_empty() {
            out.push(("hex", t));
        }
    }
    if let Ok(c) = urlencoding::decode(s) {
        let t = c.into_owned();
        if t != s {
            out.push(("url", t));
        }
    }
    let rot: String = s
        .chars()
        .map(|c| match c {
            'a'..='z' => (((c as u8 - b'a' + 13) % 26) + b'a') as char,
            'A'..='Z' => (((c as u8 - b'A' + 13) % 26) + b'A') as char,
            o => o,
        })
        .collect();
    if rot != s {
        out.push(("rot13", rot));
    }
    out
}

/// Bounded BFS over decoder chains — finds the chain that reveals a flag (or the
/// most readable result). The "smart loop" across codecs.
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?.to_string();
        let pattern = pstr(params, "pattern", r"[A-Za-z0-9_]+\{[^}]*\}");
        let depth = params.get("depth").and_then(|v| v.as_f64()).unwrap_or(8.0) as usize;
        let re = regex::Regex::new(pattern).ok();

        let mut seen: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, String, usize)> = VecDeque::new();
        seen.insert(input.clone());
        queue.push_back((input.clone(), String::new(), 0));
        let mut best = (input.clone(), String::new(), english_score(&input));

        while let Some((cur, chain, d)) = queue.pop_front() {
            if let Some(re) = &re {
                if re.is_match(&cur) {
                    return Ok(result(cur, chain, true));
                }
            }
            let score = english_score(&cur);
            if score > best.2 {
                best = (cur.clone(), chain.clone(), score);
            }
            if d >= depth || seen.len() > 4096 {
                continue;
            }
            for (name, next) in expand(&cur) {
                if seen.insert(next.clone()) {
                    let next_chain = if chain.is_empty() {
                        name.to_string()
                    } else {
                        format!("{chain} → {name}")
                    };
                    queue.push_back((next, next_chain, d + 1));
                }
            }
        }
        Ok(result(best.0, best.1, false))
    }
}

fn result(text: String, chain: String, hit: bool) -> PortMap {
    let mut out = PortMap::new();
    out.insert("text".to_string(), PortValue::Text(text));
    out.insert("chain".to_string(), PortValue::Text(chain));
    out.insert("hit".to_string(), PortValue::Bool(hit));
    out
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "magic_decode",
            ENC,
            "魔法解码",
            AMBER,
            vec![t_in()],
            vec![
                req("text", "结果", PortType::Text),
                opt("chain", "解码链", PortType::Text),
                opt("hit", "命中", PortType::Bool),
            ],
            vec![
                ParamSpec::text("pattern", "目标正则", r"flag\{[^}]*\}", false),
                ParamSpec::number("depth", "最大深度", 1.0, 16.0, 1.0, 8.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
