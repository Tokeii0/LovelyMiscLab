//! Extract a substring by character offset + length.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let chars: Vec<char> = in_text(inputs, "text")?.chars().collect();
        let start = (pnum(params, "start", 0.0) as i64).max(0) as usize;
        let len = pnum(params, "length", 0.0) as i64;
        let end = if len <= 0 {
            chars.len()
        } else {
            (start + len as usize).min(chars.len())
        };
        let out: String = chars.get(start..end).unwrap_or(&[]).iter().collect();
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "substring",
            TXT,
            "截取子串",
            TEAL,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::number("start", "起始位置", 0.0, 1_000_000.0, 1.0, 0.0),
                ParamSpec::number("length", "长度(0=到末尾)", 0.0, 1_000_000.0, 1.0, 0.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
