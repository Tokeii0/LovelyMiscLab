//! Sort lines — alphabetical / numeric / length / reverse.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut lines: Vec<&str> = in_text(inputs, "text")?.lines().collect();
        match pstr(params, "order", "字母升序") {
            "字母降序" => {
                lines.sort();
                lines.reverse();
            }
            "数字升序" => lines.sort_by(|a, b| {
                num(a)
                    .partial_cmp(&num(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            "长度升序" => lines.sort_by_key(|l| l.chars().count()),
            "反转" => lines.reverse(),
            _ => lines.sort(),
        }
        Ok(out_text(lines.join("\n")))
    }
}

fn num(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(f64::INFINITY)
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "sort_lines",
            TXT,
            "行排序",
            TEAL,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select(
                "order",
                "顺序",
                &["字母升序", "字母降序", "数字升序", "长度升序", "反转"],
                "字母升序",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
