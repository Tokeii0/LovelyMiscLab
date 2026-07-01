use super::prelude::*;

struct Decode;
impl Node for Decode {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let cleaned: String = in_text(inputs, "text")?.split_whitespace().collect();
        let bytes = hex::decode(&cleaned).map_err(|e| CoreError::Parse(format!("Hex 解码失败: {e}")))?;
        Ok(out_text(String::from_utf8_lossy(&bytes).into_owned()))
    }
}

struct Encode;
impl Node for Encode {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(out_text(hex::encode(in_text(inputs, "text")?.as_bytes())))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc("hex_decode", ENC, "Hex 解码", BLUE, vec![t_in()], vec![t_out()], vec![]),
        Arc::new(|| Arc::new(Decode)),
    );
    reg.register(
        desc("hex_encode", ENC, "Hex 编码", BLUE, vec![t_in()], vec![t_out()], vec![]),
        Arc::new(|| Arc::new(Encode)),
    );
}
