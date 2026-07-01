//! Numeric range generator — the dataflow "for i in start..end step" as a list source.
use super::prelude::*;

fn fmt_num(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        x.to_string()
    }
}

struct N;
impl Node for N {
    fn run(
        &self,
        _inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let start = pnum(params, "start", 0.0);
        let end = pnum(params, "end", 10.0);
        let step = pnum(params, "step", 1.0);
        if step == 0.0 {
            return Err(CoreError::Parse("步长不能为 0".into()));
        }
        let mut list = Vec::new();
        let mut i = start;
        while (step > 0.0 && i < end) || (step < 0.0 && i > end) {
            list.push(fmt_num(i));
            i += step;
            if list.len() >= 100_000 {
                break;
            }
        }
        let count = list.len() as f64;
        let mut m = PortMap::new();
        m.insert("list".to_string(), PortValue::StringList(list));
        m.insert("count".to_string(), PortValue::Number(count));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "range",
            CTL,
            "数值范围",
            AMBER,
            vec![],
            vec![
                req("list", "序列", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![
                ParamSpec::number("start", "起始", -1_000_000.0, 1_000_000.0, 1.0, 0.0),
                ParamSpec::number("end", "结束(不含)", -1_000_000.0, 1_000_000.0, 1.0, 10.0),
                ParamSpec::number("step", "步长", -1_000_000.0, 1_000_000.0, 1.0, 1.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
