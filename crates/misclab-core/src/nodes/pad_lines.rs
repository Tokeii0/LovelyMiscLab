//! Pad each line to a target width with a chosen character.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(&self, inputs: &PortMap, params: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let width = pnum(params, "width", 8.0).max(0.0) as usize;
        let pad = pstr(params, "char", " ").chars().next().unwrap_or(' ');
        let left = pstr(params, "side", "右侧") == "左侧";
        let out = in_text(inputs, "text")?
            .lines()
            .map(|l| {
                let n = l.chars().count();
                if n >= width {
                    l.to_string()
                } else {
                    let padding: String = std::iter::repeat_n(pad, width - n).collect();
                    if left {
                        format!("{padding}{l}")
                    } else {
                        format!("{l}{padding}")
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "pad_lines",
            TXT,
            "行填充",
            TEAL,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::number("width", "目标宽度", 0.0, 1000.0, 1.0, 8.0),
                ParamSpec::text("char", "填充字符", " ", false),
                ParamSpec::select("side", "方向", &["右侧", "左侧"], "右侧"),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
