//! Multi-way switch — a selector index picks one of `case0..case3`, else `default`.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let idx = match inputs.get("selector") {
            Some(PortValue::Number(n)) => *n as i64,
            Some(PortValue::Text(t)) => t.trim().parse::<i64>().unwrap_or(-1),
            Some(PortValue::Bool(b)) => *b as i64,
            _ => -1,
        };
        let chosen = inputs
            .get(&format!("case{idx}"))
            .or_else(|| inputs.get("default"))
            .cloned()
            .unwrap_or(PortValue::None);
        Ok(one("output", chosen))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "switch_case",
            CTL,
            "多路分支",
            AMBER,
            vec![
                req("selector", "选择器", PortType::Any),
                opt("case0", "分支0", PortType::Any),
                opt("case1", "分支1", PortType::Any),
                opt("case2", "分支2", PortType::Any),
                opt("case3", "分支3", PortType::Any),
                opt("default", "默认", PortType::Any),
            ],
            vec![req("output", "输出", PortType::Any)],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
