use super::basex::*;
use super::prelude::*;

/// Resolve the alphabet and whether it is the Standard (Ascii85) set — the only
/// one that uses the `z` all-zero shortcut and `<~ ~>` delimiters.
fn resolve(params: &serde_json::Value) -> (Vec<char>, bool) {
    match pstr(params, "variant", "标准") {
        "Z85" => (expand_alph_range(B85_Z85), false),
        "IPv6" => (expand_alph_range(B85_IPV6), false),
        _ => (expand_alph_range(B85_STANDARD), true),
    }
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let (alphabet, standard) = resolve(params);
        let delim = pbool(params, "delim", false);
        Ok(out_text(base85_encode(
            &in_bytes(inputs, "data")?,
            &alphabet,
            standard,
            delim,
        )))
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
        let (alphabet, standard) = resolve(params);
        let strip = pbool(params, "strip", true);
        let zero_char = if standard { Some('z') } else { None };
        Ok(decoded(base85_decode(
            in_text(inputs, "text")?,
            &alphabet,
            strip,
            zero_char,
        )?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let variant = || ParamSpec::select("variant", "码表", &["标准", "Z85", "IPv6"], "标准");
    reg.register(
        desc(
            "base85_encode",
            ENC,
            "Base85 编码",
            BLUE,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![variant(), ParamSpec::toggle("delim", "包含 <~ ~> 分隔符", false)],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "base85_decode",
            ENC,
            "Base85 解码",
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
