use super::prelude::*;

struct Decode;
impl Node for Decode {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let decoded = urlencoding::decode(in_text(inputs, "text")?)
            .map_err(|e| CoreError::Parse(format!("URL 解码失败: {e}")))?;
        Ok(out_text(decoded.into_owned()))
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
        Ok(out_text(urlencoding::encode(in_text(inputs, "text")?).into_owned()))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc("url_decode", ENC, "URL 解码", BLUE, vec![t_in()], vec![t_out()], vec![]),
        Arc::new(|| Arc::new(Decode)),
    );
    reg.register(
        desc("url_encode", ENC, "URL 编码", BLUE, vec![t_in()], vec![t_out()], vec![]),
        Arc::new(|| Arc::new(Encode)),
    );
}
