//! 列表归约：把列表折叠成一个值 —— 数量 / 求和 / 最小 / 最大 / 平均 / 首个 / 末个 / 连接。
//! 数值类归约额外给出 `number` 端口，可直接驱动 `数学运算`/`数值比较`。
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
        let op = pstr(p, "op", "数量");
        // 数值类归约把能 parse 成 f64 的元素挑出来（非数字跳过）。
        let nums: Vec<f64> = list.iter().filter_map(|s| s.trim().parse::<f64>().ok()).collect();

        let mut number: Option<f64> = None;
        let value: String = match op {
            "数量" => {
                let n = list.len() as f64;
                number = Some(n);
                fmt_num(n)
            }
            "求和" => {
                let s: f64 = nums.iter().sum();
                number = Some(s);
                fmt_num(s)
            }
            "最小" => match nums.iter().cloned().reduce(f64::min) {
                Some(v) => {
                    number = Some(v);
                    fmt_num(v)
                }
                None => String::new(),
            },
            "最大" => match nums.iter().cloned().reduce(f64::max) {
                Some(v) => {
                    number = Some(v);
                    fmt_num(v)
                }
                None => String::new(),
            },
            "平均" => {
                if nums.is_empty() {
                    String::new()
                } else {
                    let v = nums.iter().sum::<f64>() / nums.len() as f64;
                    number = Some(v);
                    fmt_num(v)
                }
            }
            "首个" => list.first().cloned().unwrap_or_default(),
            "末个" => list.last().cloned().unwrap_or_default(),
            "连接" => list.join(pstr(p, "sep", "")),
            _ => String::new(),
        };

        let mut m = PortMap::new();
        m.insert("value".into(), PortValue::Text(value));
        if let Some(n) = number {
            m.insert("number".into(), PortValue::Number(n));
        }
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "list_reduce",
            CTL,
            "列表归约",
            AMBER,
            vec![req("list", "列表", PortType::StringList)],
            vec![
                req("value", "值", PortType::Text),
                opt("number", "数值", PortType::Number),
            ],
            vec![
                ParamSpec::select(
                    "op",
                    "归约",
                    &["数量", "求和", "最小", "最大", "平均", "首个", "末个", "连接"],
                    "数量",
                ),
                ParamSpec::text("sep", "连接分隔符", "", false),
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

    fn reduce(items: &[&str], op: &str) -> (String, Option<f64>) {
        let mut i = PortMap::new();
        i.insert(
            "list".into(),
            PortValue::StringList(items.iter().map(|s| s.to_string()).collect()),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "list_reduce",
            &i,
            &serde_json::json!({ "op": op }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        let v = match out.get("value") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        };
        let n = match out.get("number") {
            Some(PortValue::Number(n)) => Some(*n),
            _ => None,
        };
        (v, n)
    }

    #[test]
    fn folds() {
        assert_eq!(reduce(&["1", "2", "3"], "数量"), ("3".into(), Some(3.0)));
        assert_eq!(reduce(&["1", "2", "3"], "求和"), ("6".into(), Some(6.0)));
        assert_eq!(reduce(&["3", "1", "2"], "最小"), ("1".into(), Some(1.0)));
        assert_eq!(reduce(&["3", "1", "2"], "最大"), ("3".into(), Some(3.0)));
        assert_eq!(reduce(&["2", "4"], "平均"), ("3".into(), Some(3.0)));
        assert_eq!(reduce(&["x", "y"], "首个").0, "x");
        assert_eq!(reduce(&["x", "y"], "末个").0, "y");
    }
}
