use super::basex::*;
use super::prelude::*;

fn alpha(params: &serde_json::Value) -> Vec<char> {
    expand_alph_range(match pstr(params, "variant", "标准") {
        "Hex 扩展" => B32_HEX,
        _ => B32_STANDARD,
    })
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(out_text(base32_encode(&in_bytes(inputs, "data")?, &alpha(params))))
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
        Ok(decoded(base32_decode(in_text(inputs, "text")?, &alpha(params), strip)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let variant = || ParamSpec::select("variant", "码表", &["标准", "Hex 扩展"], "标准");
    reg.register(
        desc(
            "base32_encode",
            ENC,
            "Base32 编码",
            BLUE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![variant()],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "base32_decode",
            ENC,
            "Base32 解码",
            BLUE,
            vec![t_in()],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![variant(), ParamSpec::toggle("strip", "去除非码表字符", true)],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
