use super::prelude::*;

/// Common CTF presets. "自定义" falls back to the `pattern` param.
const PRESETS: &[(&str, &str)] = &[
    ("flag", r"[A-Za-z0-9_]+\{[^}]*\}"),
    ("MD5", r"\b[a-fA-F0-9]{32}\b"),
    ("SHA1", r"\b[a-fA-F0-9]{40}\b"),
    ("IPv4", r"\b\d{1,3}(?:\.\d{1,3}){3}\b"),
    ("邮箱", r"[\w.+-]+@[\w-]+\.[\w.-]+"),
    ("URL", r"https?://[^\s]+"),
    ("Base64块", r"[A-Za-z0-9+/]{16,}={0,2}"),
    ("Hex串", r"\b[a-fA-F0-9]{8,}\b"),
];

fn resolve_pattern(p: &serde_json::Value) -> String {
    let preset = pstr(p, "preset", "自定义");
    if preset == "自定义" {
        return pstr(p, "pattern", "").to_string();
    }
    PRESETS
        .iter()
        .find(|(k, _)| *k == preset)
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?;
        let pattern = resolve_pattern(params);
        let re = regex::Regex::new(&pattern).map_err(|e| CoreError::Parse(format!("正则错误: {e}")))?;
        let matches: Vec<String> = re.find_iter(input).map(|m| m.as_str().to_string()).collect();
        let first = matches.first().cloned().unwrap_or_default();
        let mut out = PortMap::new();
        out.insert("text".to_string(), PortValue::Text(first));
        out.insert("matches".to_string(), PortValue::StringList(matches));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "regex_extract",
            TXT,
            "正则提取",
            TEAL,
            vec![t_in()],
            vec![
                req("text", "首个匹配", PortType::Text),
                opt("matches", "全部匹配", PortType::StringList),
            ],
            vec![
                ParamSpec::select(
                    "preset",
                    "预设",
                    &["自定义", "flag", "MD5", "SHA1", "IPv4", "邮箱", "URL", "Base64块", "Hex串"],
                    "flag",
                ),
                ParamSpec::text("pattern", "自定义正则", r"flag\{[^}]*\}", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
