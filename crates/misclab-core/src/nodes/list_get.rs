//! 列表取值：取列表第 `index` 项为文本（负数从末尾数，Python 风格），把列表世界桥回标量
//! 供 `比较`/`条件选择` 使用。`index` 可「转为输入」由 `数值范围`/计数驱动做遍历取值。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let list = in_list(i, "list")?;
        let len = list.len() as i64;
        let raw = pnum(p, "index", 0.0) as i64;
        let idx = if raw < 0 { len + raw } else { raw };
        let (value, found) = if idx >= 0 && idx < len {
            (list[idx as usize].clone(), true)
        } else {
            (String::new(), false)
        };
        let mut m = PortMap::new();
        m.insert("value".into(), PortValue::Text(value));
        m.insert("found".into(), PortValue::Bool(found));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "list_get",
            CTL,
            "列表取值",
            AMBER,
            vec![req("list", "列表", PortType::StringList)],
            vec![
                req("value", "值", PortType::Text),
                opt("found", "存在", PortType::Bool),
            ],
            vec![ParamSpec::number(
                "index",
                "索引(负=从末尾)",
                -1_000_000.0,
                1_000_000.0,
                1.0,
                0.0,
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

    fn get(index: f64) -> (String, bool) {
        let mut i = PortMap::new();
        i.insert(
            "list".into(),
            PortValue::StringList(vec!["x".into(), "y".into(), "z".into()]),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "list_get",
            &i,
            &serde_json::json!({ "index": index }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        let v = match out.get("value") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        };
        let f = matches!(out.get("found"), Some(PortValue::Bool(true)));
        (v, f)
    }

    #[test]
    fn positive_negative_and_oob() {
        assert_eq!(get(1.0), ("y".into(), true));
        assert_eq!(get(-1.0), ("z".into(), true)); // last
        assert_eq!(get(5.0), (String::new(), false)); // out of bounds
    }
}
