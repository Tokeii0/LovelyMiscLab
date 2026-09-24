//! A value picker. Emits its `value` param on the `value` port (Text). Wire it
//! into another node's promoted select-parameter (e.g. 哈希计算 的「算法」); the
//! frontend pulls that parameter's option list into the selector's dropdown so
//! you choose a validated value, and one selector can drive several nodes.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        _inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(one(
            "value",
            PortValue::Text(pstr(params, "value", "").to_string()),
        ))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "selector",
            CTL,
            "选择器",
            AMBER,
            vec![],
            vec![req("value", "值", PortType::Text)],
            vec![ParamSpec::text("value", "值", "", false)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
