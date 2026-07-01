use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let len = in_text(inputs, "text")?.chars().count() as f64;
        Ok(one("length", PortValue::Number(len)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "length",
            TXT,
            "文本长度",
            TEAL,
            vec![t_in()],
            vec![req("length", "长度", PortType::Number)],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
