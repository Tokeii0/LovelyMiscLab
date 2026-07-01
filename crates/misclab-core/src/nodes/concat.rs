use super::prelude::*;

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
        let sep = pstr(params, "sep", "");
        Ok(out_text(format!("{a}{sep}{b}")))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "concat",
            TXT,
            "文本合并",
            TEAL,
            vec![req("a", "A", PortType::Text), req("b", "B", PortType::Text)],
            vec![t_out()],
            vec![ParamSpec::text("sep", "分隔符", "", false)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
