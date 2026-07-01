//! Conditional gate — forward the value only when the condition is true, else None.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let pass = matches!(inputs.get("condition"), Some(PortValue::Bool(true)));
        let output = if pass {
            inputs.get("value").cloned().unwrap_or(PortValue::None)
        } else {
            PortValue::None
        };
        let mut m = PortMap::new();
        m.insert("output".to_string(), output);
        m.insert("passed".to_string(), PortValue::Bool(pass));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "gate",
            CTL,
            "条件门",
            AMBER,
            vec![
                req("value", "值", PortType::Any),
                req("condition", "条件", PortType::Bool),
            ],
            vec![
                req("output", "输出", PortType::Any),
                opt("passed", "已通过", PortType::Bool),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
