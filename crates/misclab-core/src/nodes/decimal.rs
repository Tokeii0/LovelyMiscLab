//! Bytes ↔ decimal char codes ("104 101 108 …"). Common for charcode challenges.
use super::basex::decoded;
use super::prelude::*;

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let sep = if pstr(params, "delimiter", "空格") == "逗号" { "," } else { " " };
        let s = data.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(sep);
        Ok(out_text(s))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut out = Vec::new();
        for tok in in_text(inputs, "text")?
            .split(|c: char| !c.is_ascii_digit())
            .filter(|t| !t.is_empty())
        {
            let n: u32 = tok.parse().map_err(|_| CoreError::Parse(format!("非法数字: {tok}")))?;
            if n > 255 {
                return Err(CoreError::Parse(format!("字节值超出范围(0-255): {n}")));
            }
            out.push(n as u8);
        }
        Ok(decoded(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "to_decimal",
            RADIX,
            "转十进制",
            SLATE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![ParamSpec::select("delimiter", "分隔符", &["空格", "逗号"], "空格")],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "from_decimal",
            RADIX,
            "十进制转文本",
            SLATE,
            vec![t_in()],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
