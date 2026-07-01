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
        Ok(out_text(base92_encode(&in_bytes(inputs, "data")?)))
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
        Ok(decoded(base92_decode(in_text(inputs, "text")?)?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "base92_encode",
            ENC,
            "Base92 编码",
            BLUE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "base92_decode",
            ENC,
            "Base92 解码",
            BLUE,
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
