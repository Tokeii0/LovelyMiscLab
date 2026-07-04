//! 注释：画布上的说明便签（CyberChef「Comment」式）。无输入无输出，纯标注；因为没有输入
//! 端口，`GenericNode` 会把多行文本参数直接内联渲染成可编辑的便签框。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        _i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(PortMap::new())
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "note",
            CTL,
            "注释",
            SLATE,
            vec![],
            vec![],
            vec![ParamSpec::text("text", "备注", "", true)],
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

    #[test]
    fn runs_to_empty_output() {
        let out = GraphExecutor::run_node(
            &default_registry(),
            "note",
            &PortMap::new(),
            &serde_json::json!({ "text": "记一笔" }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(out.is_empty());
    }
}
