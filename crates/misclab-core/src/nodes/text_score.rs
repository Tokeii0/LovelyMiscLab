use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?;
        let total = input.chars().count().max(1);
        let printable = input
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\n' | '\t' | '\r'))
            .count();
        let score = printable as f64 / total as f64;
        let flag = regex::Regex::new(r"[A-Za-z0-9_]+\{[^}\n]{0,256}\}")
            .ok()
            .and_then(|re| re.find(input).map(|m| m.as_str().to_string()))
            .unwrap_or_default();
        let mut out = PortMap::new();
        out.insert("score".to_string(), PortValue::Number(score));
        out.insert("flag".to_string(), PortValue::Text(flag));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "text_score",
            TXT,
            "文本评分",
            TEAL,
            vec![t_in()],
            vec![
                req("score", "可读性", PortType::Number),
                opt("flag", "疑似 flag", PortType::Text),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
