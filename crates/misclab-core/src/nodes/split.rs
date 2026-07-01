use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?;
        let sep = pstr(params, "sep", ",");
        let parts: Vec<String> = if sep.is_empty() {
            input.chars().map(|c| c.to_string()).collect()
        } else {
            input.split(sep).map(|s| s.to_string()).collect()
        };
        Ok(one("list", PortValue::StringList(parts)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "split",
            TXT,
            "文本分割",
            TEAL,
            vec![t_in()],
            vec![req("list", "列表", PortType::StringList)],
            vec![ParamSpec::text("sep", "分隔符", ",", false)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
