//! 列表操作：对列表做 反转 / 切片 / 取前N / 跳过N / 去尾N（元素顺序层面）。
//! 注意「反转」是**列表元素顺序**反转，与文本处理里的「反转」(字符串反转) 不同。
//! 排序/去重请用文本处理里的 `行排序`/`行去重`。
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
        let op = pstr(p, "op", "反转");
        let start = pnum(p, "start", 0.0).max(0.0) as usize;
        let n = pnum(p, "count", 10.0).max(0.0) as usize;
        let len = list.len();
        let out: Vec<String> = match op {
            "反转" => list.into_iter().rev().collect(),
            "切片" => {
                let s = start.min(len);
                let e = start.saturating_add(n).min(len);
                list[s..e].to_vec()
            }
            "取前N" => list.into_iter().take(n).collect(),
            "跳过N" => list.into_iter().skip(n).collect(),
            "去尾N" => {
                let keep = len.saturating_sub(n);
                list.into_iter().take(keep).collect()
            }
            _ => list,
        };
        let count = out.len() as f64;
        let mut m = PortMap::new();
        m.insert("list".into(), PortValue::StringList(out));
        m.insert("count".into(), PortValue::Number(count));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "list_ops",
            CTL,
            "列表操作",
            AMBER,
            vec![req("list", "列表", PortType::StringList)],
            vec![
                req("list", "结果", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![
                ParamSpec::select(
                    "op",
                    "操作",
                    &["反转", "切片", "取前N", "跳过N", "去尾N"],
                    "反转",
                ),
                ParamSpec::number("start", "起点(切片)", 0.0, 1_000_000.0, 1.0, 0.0),
                ParamSpec::number("count", "数量/N", 0.0, 1_000_000.0, 1.0, 10.0),
            ],
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

    fn run(op: &str, start: f64, count: f64) -> Vec<String> {
        let mut i = PortMap::new();
        i.insert(
            "list".into(),
            PortValue::StringList(vec!["a".into(), "b".into(), "c".into(), "d".into()]),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "list_ops",
            &i,
            &serde_json::json!({ "op": op, "start": start, "count": count }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("list") {
            Some(PortValue::StringList(v)) => v.clone(),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn ops() {
        assert_eq!(run("反转", 0.0, 0.0), vec!["d", "c", "b", "a"]);
        assert_eq!(run("切片", 1.0, 2.0), vec!["b", "c"]);
        assert_eq!(run("取前N", 0.0, 2.0), vec!["a", "b"]);
        assert_eq!(run("跳过N", 0.0, 2.0), vec!["c", "d"]);
        assert_eq!(run("去尾N", 0.0, 1.0), vec!["a", "b", "c"]);
        assert_eq!(run("切片", 10.0, 5.0), Vec::<String>::new()); // out of bounds → empty
    }
}
