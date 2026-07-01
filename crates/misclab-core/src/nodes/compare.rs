use super::prelude::*;

/// Compare two texts with a chosen operator → Bool (feeds a `switch`).
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let a = in_text(inputs, "a")?;
        let b = in_text(inputs, "b")?;
        let result = match pstr(params, "op", "==") {
            "==" => a == b,
            "!=" => a != b,
            "包含" => a.contains(b),
            "开头" => a.starts_with(b),
            "结尾" => a.ends_with(b),
            "匹配正则" => regex::Regex::new(b).map(|re| re.is_match(a)).unwrap_or(false),
            _ => false,
        };
        Ok(one("result", PortValue::Bool(result)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "compare",
            CTL,
            "比较",
            AMBER,
            vec![req("a", "A", PortType::Text), req("b", "B", PortType::Text)],
            vec![req("result", "结果", PortType::Bool)],
            vec![ParamSpec::select(
                "op",
                "运算",
                &["==", "!=", "包含", "开头", "结尾", "匹配正则"],
                "==",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
