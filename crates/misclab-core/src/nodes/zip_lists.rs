//! 列表配对：把 A[i]、B[i] 拼成 `A[i]{sep}B[i]`。`最短` 按短的截断，`最长` 用空串补齐。
//! 常用于拼 `user:pass` 之类，喂给下游 `逐项映射`/`通用口令爆破`。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let a = in_list(i, "a")?;
        let b = in_list(i, "b")?;
        let sep = pstr(p, "sep", ":");
        let longest = pstr(p, "length", "最短") == "最长";
        let n = if longest {
            a.len().max(b.len())
        } else {
            a.len().min(b.len())
        };
        let out: Vec<String> = (0..n)
            .map(|k| {
                let x = a.get(k).map(String::as_str).unwrap_or("");
                let y = b.get(k).map(String::as_str).unwrap_or("");
                format!("{x}{sep}{y}")
            })
            .collect();
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
            "zip_lists",
            CTL,
            "列表配对",
            AMBER,
            vec![
                req("a", "列表A", PortType::StringList),
                req("b", "列表B", PortType::StringList),
            ],
            vec![
                req("list", "结果", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![
                ParamSpec::text("sep", "配对分隔符", ":", false),
                ParamSpec::select("length", "长度", &["最短", "最长"], "最短"),
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

    fn zip(a: &[&str], b: &[&str], length: &str) -> Vec<String> {
        let mut i = PortMap::new();
        i.insert(
            "a".into(),
            PortValue::StringList(a.iter().map(|s| s.to_string()).collect()),
        );
        i.insert(
            "b".into(),
            PortValue::StringList(b.iter().map(|s| s.to_string()).collect()),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "zip_lists",
            &i,
            &serde_json::json!({ "sep": ":", "length": length }),
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
    fn shortest_and_longest() {
        assert_eq!(zip(&["u1", "u2"], &["p1", "p2", "p3"], "最短"), vec!["u1:p1", "u2:p2"]);
        assert_eq!(
            zip(&["u1"], &["p1", "p2"], "最长"),
            vec!["u1:p1", ":p2"] // padded with empty
        );
    }
}
