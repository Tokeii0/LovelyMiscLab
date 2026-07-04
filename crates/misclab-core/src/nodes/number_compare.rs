//! 数值比较：对两个数值做 == != > >= < <= 比较 → 布尔。补 `compare`（只能比文本）的
//! 缺口，让分支能由 `range`/计数/`math` 等数值驱动（把 result 接到 条件门/条件选择）。
use super::prelude::*;

struct N;
impl Node for N {
    #[allow(clippy::float_cmp)] // 数值比较节点，== / != 就是要按精确相等语义
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let a = in_num(i, "a").ok_or_else(|| CoreError::MissingInput("a".into()))?;
        let b = in_num(i, "b").ok_or_else(|| CoreError::MissingInput("b".into()))?;
        let result = match pstr(p, "op", "==") {
            "==" => a == b,
            "!=" => a != b,
            ">" => a > b,
            ">=" => a >= b,
            "<" => a < b,
            "<=" => a <= b,
            _ => false,
        };
        Ok(one("result", PortValue::Bool(result)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "number_compare",
            CTL,
            "数值比较",
            AMBER,
            vec![req("a", "A", PortType::Number), req("b", "B", PortType::Number)],
            vec![req("result", "结果", PortType::Bool)],
            vec![ParamSpec::select(
                "op",
                "运算",
                &["==", "!=", ">", ">=", "<", "<="],
                "==",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    fn cmp(a: f64, b: f64, op: &str) -> bool {
        let mut i = PortMap::new();
        i.insert("a".into(), PortValue::Number(a));
        i.insert("b".into(), PortValue::Number(b));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "number_compare",
            &i,
            &serde_json::json!({ "op": op }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        matches!(out.get("result"), Some(PortValue::Bool(true)))
    }

    #[test]
    fn operators() {
        assert!(cmp(3.0, 3.0, "=="));
        assert!(!cmp(3.0, 4.0, "=="));
        assert!(cmp(5.0, 2.0, ">"));
        assert!(cmp(2.0, 2.0, ">="));
        assert!(cmp(1.0, 2.0, "<"));
        assert!(cmp(2.0, 2.0, "<="));
        assert!(cmp(1.0, 2.0, "!="));
    }
}
