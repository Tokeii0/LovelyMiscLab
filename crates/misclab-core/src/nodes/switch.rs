use super::prelude::*;

/// Route: outputs `a` when the boolean condition is true, else `b`.
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let cond = matches!(inputs.get("condition"), Some(PortValue::Bool(true)));
        let chosen = if cond {
            inputs.get("a")
        } else {
            inputs.get("b")
        }
        .cloned()
        .unwrap_or(PortValue::None);
        Ok(one("output", chosen))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "switch",
            CTL,
            "条件选择",
            AMBER,
            vec![
                req("condition", "条件", PortType::Bool),
                req("a", "真", PortType::Any),
                req("b", "假", PortType::Any),
            ],
            vec![req("output", "输出", PortType::Any)],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
