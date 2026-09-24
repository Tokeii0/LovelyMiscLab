//! Bytes ↔ octal representation.
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
        let sep = if pstr(params, "delimiter", "空格") == "无" {
            ""
        } else {
            " "
        };
        let s = data
            .iter()
            .map(|b| format!("{b:03o}"))
            .collect::<Vec<_>>()
            .join(sep);
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
            .split(|c: char| !('0'..='7').contains(&c))
            .filter(|t| !t.is_empty())
        {
            let n = u32::from_str_radix(tok, 8)
                .map_err(|_| CoreError::Parse(format!("非法八进制: {tok}")))?;
            if n > 255 {
                return Err(CoreError::Parse(format!("字节超范围: {tok}")));
            }
            out.push(n as u8);
        }
        Ok(decoded(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "to_octal",
            RADIX,
            "转八进制",
            SLATE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![ParamSpec::select(
                "delimiter",
                "分隔符",
                &["空格", "无"],
                "空格",
            )],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "from_octal",
            RADIX,
            "八进制转文本",
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
