//! Extract well-known tokens (IPs, emails, URLs, MACs…) from noisy text.
//! Mirrors CyberChef's "Extract IP addresses / URLs / email addresses / MAC
//! addresses" family, collapsed into one node with a `kind` selector.
use super::prelude::*;

const KINDS: &[(&str, &str)] = &[
    ("IPv4", r"\b(?:\d{1,3}\.){3}\d{1,3}\b"),
    ("IPv6", r"\b(?:[A-Fa-f0-9]{1,4}:){2,7}[A-Fa-f0-9]{1,4}\b"),
    ("邮箱", r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}"),
    ("URL", r#"[A-Za-z][A-Za-z0-9+.\-]*://[^\s"'<>]+"#),
    ("MAC地址", r"\b(?:[0-9A-Fa-f]{2}[:\-]){5}[0-9A-Fa-f]{2}\b"),
    (
        "域名",
        r"\b(?:[A-Za-z0-9](?:[A-Za-z0-9\-]{0,61}[A-Za-z0-9])?\.)+[A-Za-z]{2,}\b",
    ),
    ("flag", r"[A-Za-z0-9_]+\{[^}]*\}"),
    ("Base64块", r"[A-Za-z0-9+/]{16,}={0,2}"),
    ("Hex串", r"\b[A-Fa-f0-9]{8,}\b"),
];

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?;
        let kind = pstr(params, "kind", "IPv4");
        let pat = KINDS
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, v)| *v)
            .ok_or_else(|| CoreError::Parse(format!("未知提取类型: {kind}")))?;
        let re = regex::Regex::new(pat).map_err(|e| CoreError::Parse(format!("正则错误: {e}")))?;
        let mut seen = std::collections::HashSet::new();
        let mut matches: Vec<String> = Vec::new();
        for m in re.find_iter(input) {
            let s = m.as_str().to_string();
            if pbool(params, "unique", true) {
                if seen.insert(s.clone()) {
                    matches.push(s);
                }
            } else {
                matches.push(s);
            }
        }
        let mut out = PortMap::new();
        out.insert("text".to_string(), PortValue::Text(matches.join("\n")));
        out.insert("count".to_string(), PortValue::Number(matches.len() as f64));
        out.insert("matches".to_string(), PortValue::StringList(matches));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let kinds: Vec<&str> = KINDS.iter().map(|(k, _)| *k).collect();
    reg.register(
        desc(
            "extract",
            UTIL,
            "信息提取",
            AMBER,
            vec![t_in()],
            vec![
                req("text", "匹配(每行一个)", PortType::Text),
                opt("matches", "列表", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![
                ParamSpec::select("kind", "提取类型", &kinds, "IPv4"),
                ParamSpec::toggle("unique", "去重", true),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
