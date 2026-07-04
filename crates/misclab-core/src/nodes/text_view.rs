//! 文本展示：把上游文本原样透传，并在节点上多行展示。既有输入又有输出，可插在数据流
//! 中间做「监视 + 继续」——区别于只进不出的「文本输出」。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text").unwrap_or("").to_string();
        Ok(out_text(text))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        {
            let mut d = desc(
                "text_view",
                IO,
                "文本展示",
                GREEN,
                vec![req("text", "文本", PortType::Text)],
                vec![t_out()],
                vec![],
            );
            d.description = "把上游文本原样透传并在节点上多行展示，可插在流程中间做监视。".into();
            d
        },
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

    #[test]
    fn passes_text_through() {
        let mut inputs = PortMap::new();
        inputs.insert("text".into(), PortValue::Text("flag{view}\nline2".into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "text_view",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("text") {
            Some(PortValue::Text(s)) => assert_eq!(s, "flag{view}\nline2"),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn missing_input_yields_empty() {
        let out = GraphExecutor::run_node(
            &default_registry(),
            "text_view",
            &PortMap::new(),
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("text"), Some(PortValue::Text(s)) if s.is_empty()));
    }
}
