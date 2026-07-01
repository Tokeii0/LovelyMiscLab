use super::basex::*;
use super::prelude::*;

fn alpha(params: &serde_json::Value) -> Result<Vec<char>, CoreError> {
    let a = expand_alph_range(pstr(params, "alphabet", B62_STANDARD));
    if a.len() < 2 {
        return Err(CoreError::Parse("Base62 码表至少需要 2 个字符".into()));
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
        Ok(out_text(radix_encode(&in_bytes(inputs, "data")?, &alpha(params)?, false)))
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
        // Base62 always drops non-alphabet chars (CyberChef has no toggle here).
        Ok(decoded(radix_decode(
            in_text(inputs, "text")?,
            &alpha(params)?,
            false,
            true,
        )?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let alphabet = || ParamSpec::text("alphabet", "码表", B62_STANDARD, false);
    reg.register(
        desc(
            "base62_encode",
            ENC,
            "Base62 编码",
            BLUE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![alphabet()],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "base62_decode",
            ENC,
            "Base62 解码",
            BLUE,
            vec![t_in()],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![alphabet()],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
