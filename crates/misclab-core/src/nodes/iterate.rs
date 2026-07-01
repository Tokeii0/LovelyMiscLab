//! Iterate (while) — repeatedly apply a transform until a regex matches, the value
//! stops changing, the transform fails, or the max-iteration cap. The loop lives
//! inside the node (the executor is a DAG and rejects graph cycles).
use super::prelude::*;
use super::xform::{apply_transform, TRANSFORMS};

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut cur = in_text(inputs, "text")?.to_string();
        let op = pstr(params, "op", "Base64解码");
        let max = pnum(params, "max", 16.0).max(0.0) as usize;
        let until = pstr(params, "until", "");
        let re = if until.is_empty() {
            None
        } else {
            Some(regex::Regex::new(until).map_err(|e| CoreError::Parse(format!("正则错误: {e}")))?)
        };

        let mut iters = 0usize;
        let mut hit = false;
        for _ in 0..max {
            if let Some(re) = &re {
                if re.is_match(&cur) {
                    hit = true;
                    break;
                }
            }
            match apply_transform(op, &cur) {
                Ok(next) if next != cur => {
                    cur = next;
                    iters += 1;
                }
                _ => break, // transform failed or fixed point → stop
            }
        }
        if !hit {
            if let Some(re) = &re {
                hit = re.is_match(&cur);
            }
        }

        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(cur));
        m.insert("iterations".to_string(), PortValue::Number(iters as f64));
        m.insert("hit".to_string(), PortValue::Bool(hit));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "iterate",
            CTL,
            "迭代循环",
            AMBER,
            vec![req("text", "输入", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("iterations", "迭代次数", PortType::Number),
                opt("hit", "命中", PortType::Bool),
            ],
            vec![
                ParamSpec::select("op", "操作", TRANSFORMS, "Base64解码"),
                ParamSpec::text("until", "停止正则(可选)", r"flag\{[^}]*\}", false),
                ParamSpec::number("max", "最大次数", 1.0, 100.0, 1.0, 16.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
