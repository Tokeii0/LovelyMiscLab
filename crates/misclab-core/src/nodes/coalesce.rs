//! 首个有效值：按 a→d 顺序输出第一个「有值」的输入（跳过 None 与空文本/空列表/空字节），
//! `index` 报来源序号。这是缺失的「合流」原语——`条件选择`/`条件门` 在死分支产出 None，
//! 用它把多路分支合回一路。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut m = PortMap::new();
        for (idx, name) in ["a", "b", "c", "d"].iter().enumerate() {
            if let Some(v) = i.get(*name) {
                if !port_is_blank(v) {
                    m.insert("output".into(), v.clone());
                    m.insert("index".into(), PortValue::Number(idx as f64));
                    return Ok(m);
                }
            }
        }
        m.insert("output".into(), PortValue::None);
        m.insert("index".into(), PortValue::Number(-1.0));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "coalesce",
            CTL,
            "首个有效值",
            AMBER,
            vec![
                opt("a", "值1", PortType::Any),
                opt("b", "值2", PortType::Any),
                opt("c", "值3", PortType::Any),
                opt("d", "值4", PortType::Any),
            ],
            vec![
                req("output", "输出", PortType::Any),
                opt("index", "来源序号", PortType::Number),
            ],
            vec![],
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
    fn picks_first_non_blank() {
        let mut i = PortMap::new();
        i.insert("a".into(), PortValue::None);
        i.insert("b".into(), PortValue::Text(String::new())); // blank → skip
        i.insert("c".into(), PortValue::Text("hit".into()));
        i.insert("d".into(), PortValue::Text("late".into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "coalesce",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("output"), Some(PortValue::Text(s)) if s == "hit"));
        assert!(matches!(out.get("index"), Some(PortValue::Number(n)) if *n == 2.0));
    }

    #[test]
    fn all_blank_yields_none() {
        let out = GraphExecutor::run_node(
            &default_registry(),
            "coalesce",
            &PortMap::new(),
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("output"), Some(PortValue::None)));
        assert!(matches!(out.get("index"), Some(PortValue::Number(n)) if *n == -1.0));
    }
}
