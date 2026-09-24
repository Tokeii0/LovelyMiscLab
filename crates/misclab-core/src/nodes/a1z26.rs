//! A1Z26 cipher — letters ↔ their alphabet position (A=1 … Z=26).
use super::prelude::*;

fn sep(params: &serde_json::Value) -> &'static str {
    match pstr(params, "delimiter", "空格") {
        "逗号" => ",",
        "短横" => "-",
        _ => " ",
    }
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let nums: Vec<String> = in_text(inputs, "text")?
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| (c.to_ascii_uppercase() as u8 - b'A' + 1).to_string())
            .collect();
        Ok(out_text(nums.join(sep(params))))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let out: String = in_text(inputs, "text")?
            .split(|c: char| !c.is_ascii_digit())
            .filter(|t| !t.is_empty())
            .filter_map(|t| t.parse::<u8>().ok())
            .filter(|&n| (1..=26).contains(&n))
            .map(|n| (b'A' + n - 1) as char)
            .collect();
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let delim = || ParamSpec::select("delimiter", "分隔符", &["空格", "逗号", "短横"], "空格");
    reg.register(
        desc(
            "a1z26_encode",
            ENC,
            "A1Z26 编码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![delim()],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "a1z26_decode",
            ENC,
            "A1Z26 解码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
