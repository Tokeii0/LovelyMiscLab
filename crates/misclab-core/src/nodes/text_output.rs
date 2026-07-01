use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        // Terminal display node: echo the incoming value so it can be shown inline.
        let value = inputs.get("text").cloned().unwrap_or(PortValue::None);
        Ok(one("value", value))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "text_output",
            IO,
            "文本输出",
            GREEN,
            vec![req("text", "文本", PortType::Text)],
            vec![],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
