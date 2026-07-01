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
        let from = pstr(params, "from", "");
        let to = pstr(params, "to", "");
        Ok(out_text(if from.is_empty() {
            input.to_string()
        } else {
            input.replace(from, to)
        }))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "replace",
            TXT,
            "文本替换",
            TEAL,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::text("from", "查找", "", false),
                ParamSpec::text("to", "替换为", "", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
