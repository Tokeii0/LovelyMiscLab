use super::basex::*;
use super::prelude::*;

fn alpha(params: &serde_json::Value) -> Result<Vec<char>, CoreError> {
    let s = match pstr(params, "variant", "Bitcoin") {
        "Ripple" => B58_RIPPLE,
        "自定义" => pstr(params, "alphabet", B58_BITCOIN),
        _ => B58_BITCOIN,
    };
    let a = expand_alph_range(s);
    if a.len() != 58 {
        return Err(CoreError::Parse("Base58 码表必须为 58 个字符".into()));
    }
    Ok(a)
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(out_text(radix_encode(&in_bytes(inputs, "data")?, &alpha(params)?, true)))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let strip = pbool(params, "strip", true);
        Ok(decoded(radix_decode(
            in_text(inputs, "text")?,
            &alpha(params)?,
            true,
            strip,
        )?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let variant = || ParamSpec::select("variant", "码表", &["Bitcoin", "Ripple", "自定义"], "Bitcoin");
    let custom = || ParamSpec::text("alphabet", "自定义码表(58字符)", "", false);
    reg.register(
        desc(
            "base58_encode",
            ENC,
            "Base58 编码",
            BLUE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![variant(), custom()],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "base58_decode",
            ENC,
            "Base58 解码",
            BLUE,
            vec![t_in()],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![variant(), custom(), ParamSpec::toggle("strip", "去除非码表字符", true)],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
