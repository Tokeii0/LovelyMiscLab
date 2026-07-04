//! 空值判断：判断输入是否「空」（None / 空文本(可选去空白) / 空列表 / 空字节）→ 布尔。
//! 配合 `条件门`/`条件选择` 做「上游有产出才继续」。`present = !empty`。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let trim = pbool(p, "trim", true);
        let empty = match i.get("value") {
            None => true,
            Some(PortValue::Text(s)) => {
                if trim {
                    s.trim().is_empty()
                } else {
                    s.is_empty()
                }
            }
            Some(v) => port_is_blank(v),
        };
        let mut m = PortMap::new();
        m.insert("empty".into(), PortValue::Bool(empty));
        m.insert("present".into(), PortValue::Bool(!empty));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "is_empty",
            CTL,
            "空值判断",
            AMBER,
            vec![req("value", "值", PortType::Any)],
            vec![
                req("empty", "为空", PortType::Bool),
                opt("present", "存在", PortType::Bool),
            ],
            vec![ParamSpec::toggle("trim", "去空白后判断", true)],
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

    fn is_empty(v: PortValue, trim: bool) -> bool {
        let mut i = PortMap::new();
        i.insert("value".into(), v);
        let out = GraphExecutor::run_node(
            &default_registry(),
            "is_empty",
            &i,
            &serde_json::json!({ "trim": trim }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        matches!(out.get("empty"), Some(PortValue::Bool(true)))
    }

    #[test]
    fn detects_blank() {
        assert!(is_empty(PortValue::None, true));
        assert!(is_empty(PortValue::Text("   ".into()), true));
        assert!(!is_empty(PortValue::Text("   ".into()), false));
        assert!(!is_empty(PortValue::Text("x".into()), true));
        assert!(is_empty(PortValue::StringList(vec![]), true));
    }
}
