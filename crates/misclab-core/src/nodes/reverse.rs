use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(out_text(in_text(inputs, "text")?.chars().rev().collect()))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc("reverse", TXT, "文本反转", TEAL, vec![t_in()], vec![t_out()], vec![]),
        Arc::new(|| Arc::new(N)),
    );
}
