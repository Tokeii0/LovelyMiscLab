use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        _inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(out_text(pstr(params, "text", "").to_string()))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "text_input",
            IO,
            "文本输入",
            SLATE,
            vec![],
            vec![t_out()],
            vec![ParamSpec::text("text", "文本", "", true)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
