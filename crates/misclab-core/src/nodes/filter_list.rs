//! Filter a list by regex — keep or exclude matching elements.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let list = in_list(inputs, "list")?;
        let re = regex::Regex::new(pstr(params, "pattern", "."))
            .map_err(|e| CoreError::Parse(format!("正则错误: {e}")))?;
        let keep = pstr(params, "mode", "保留匹配") == "保留匹配";
        let out: Vec<String> = list.into_iter().filter(|s| re.is_match(s) == keep).collect();
        let count = out.len() as f64;
        let mut m = PortMap::new();
        m.insert("list".to_string(), PortValue::StringList(out));
        m.insert("count".to_string(), PortValue::Number(count));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "filter_list",
            CTL,
            "列表过滤",
            AMBER,
            vec![req("list", "列表", PortType::StringList)],
            vec![
                req("list", "结果", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![
                ParamSpec::text("pattern", "正则", ".", false),
                ParamSpec::select("mode", "模式", &["保留匹配", "排除匹配"], "保留匹配"),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
