use super::prelude::*;
use base64::Engine as _;

struct Decode;
impl Node for Decode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let cleaned: String = in_text(inputs, "text")?.split_whitespace().collect();
        let bytes = base64_engine(params)?
            .decode(cleaned.as_bytes())
            .map_err(|e| CoreError::Parse(format!("Base64 解码失败: {e}")))?;
        Ok(out_text(String::from_utf8_lossy(&bytes).into_owned()))
    }
}

struct Encode;
impl Node for Encode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(out_text(base64_engine(params)?.encode(in_text(inputs, "text")?.as_bytes())))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc("base64_decode", ENC, "Base64 解码", BLUE, vec![t_in()], vec![t_out()], base64_params()),
        Arc::new(|| Arc::new(Decode)),
    );
    reg.register(
        desc("base64_encode", ENC, "Base64 编码", BLUE, vec![t_in()], vec![t_out()], base64_params()),
        Arc::new(|| Arc::new(Encode)),
    );
}
