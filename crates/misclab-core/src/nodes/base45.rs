use super::basex::*;
use super::prelude::*;

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let alphabet = expand_alph_range(B45_ALPHABET);
        Ok(out_text(base45_encode(&in_bytes(inputs, "data")?, &alphabet)))
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
        let alphabet = expand_alph_range(B45_ALPHABET);
        let strip = pbool(params, "strip", true);
        Ok(decoded(base45_decode(in_text(inputs, "text")?, &alphabet, strip)?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "base45_encode",
            ENC,
            "Base45 编码",
            BLUE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "base45_decode",
            ENC,
            "Base45 解码",
            BLUE,
            vec![t_in()],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![ParamSpec::toggle("strip", "去除非码表字符", true)],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
