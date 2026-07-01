//! Boolean logic gate — combine conditions (AND/OR/NOT/XOR/NAND/NOR).
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let a = matches!(inputs.get("a"), Some(PortValue::Bool(true)));
        let b = matches!(inputs.get("b"), Some(PortValue::Bool(true)));
        let r = match pstr(params, "op", "AND") {
            "OR" => a || b,
            "NOT" => !a,
            "XOR" => a ^ b,
            "NAND" => !(a && b),
            "NOR" => !(a || b),
            _ => a && b,
        };
        Ok(one("result", PortValue::Bool(r)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "logic",
            CTL,
            "逻辑运算",
            AMBER,
            vec![
                req("a", "A", PortType::Bool),
                opt("b", "B", PortType::Bool),
            ],
            vec![req("result", "结果", PortType::Bool)],
            vec![ParamSpec::select(
                "op",
                "运算",
                &["AND", "OR", "NOT", "XOR", "NAND", "NOR"],
                "AND",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
