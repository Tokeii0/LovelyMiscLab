//! 数学运算：对数值做 + - × ÷ mod pow min max（二元用 a,b）以及 abs neg round
//! floor ceil（一元，忽略 b）→ 数值。可算偏移/计数，配合 `range`/`number_compare`。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let a = in_num(i, "a").ok_or_else(|| CoreError::MissingInput("a".into()))?;
        let b = in_num(i, "b").unwrap_or(0.0);
        let op = pstr(p, "op", "+");
        let result = match op {
            "+" => a + b,
            "-" => a - b,
            "×" => a * b,
            "÷" => {
                if b == 0.0 {
                    return Err(CoreError::Parse("除数不能为 0".into()));
                }
                a / b
            }
            "mod" => {
                if b == 0.0 {
                    return Err(CoreError::Parse("模数不能为 0".into()));
                }
                a % b
            }
            "pow" => a.powf(b),
            "min" => a.min(b),
            "max" => a.max(b),
            "abs" => a.abs(),
            "neg" => -a,
            "round" => a.round(),
            "floor" => a.floor(),
            "ceil" => a.ceil(),
            _ => return Err(CoreError::Parse(format!("未知运算: {op}"))),
        };
        Ok(one("result", PortValue::Number(result)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "math",
            CTL,
            "数学运算",
            AMBER,
            vec![
                req("a", "A", PortType::Number),
                opt("b", "B", PortType::Number),
            ],
            vec![req("result", "结果", PortType::Number)],
            vec![ParamSpec::select(
                "op",
                "运算",
                &[
                    "+", "-", "×", "÷", "mod", "pow", "min", "max", "abs", "neg", "round", "floor",
                    "ceil",
                ],
                "+",
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

    fn calc(a: f64, b: Option<f64>, op: &str) -> Result<f64, CoreError> {
        let mut i = PortMap::new();
        i.insert("a".into(), PortValue::Number(a));
        if let Some(b) = b {
            i.insert("b".into(), PortValue::Number(b));
        }
        let out = GraphExecutor::run_node(
            &default_registry(),
            "math",
            &i,
            &serde_json::json!({ "op": op }),
            &NullSink,
            &CancellationToken::new(),
        )?;
        Ok(match out.get("result") {
            Some(PortValue::Number(n)) => *n,
            o => panic!("{o:?}"),
        })
    }

    #[test]
    fn binary_and_unary() {
        assert_eq!(calc(2.0, Some(3.0), "+").unwrap(), 5.0);
        assert_eq!(calc(10.0, Some(4.0), "-").unwrap(), 6.0);
        assert_eq!(calc(6.0, Some(7.0), "×").unwrap(), 42.0);
        assert_eq!(calc(9.0, Some(2.0), "÷").unwrap(), 4.5);
        assert_eq!(calc(-5.0, None, "abs").unwrap(), 5.0);
        assert_eq!(calc(2.4, None, "floor").unwrap(), 2.0);
    }

    #[test]
    fn divide_by_zero_errors() {
        assert!(calc(1.0, Some(0.0), "÷").is_err());
        assert!(calc(1.0, Some(0.0), "mod").is_err());
    }
}
