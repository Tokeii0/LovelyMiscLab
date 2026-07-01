//! RC4 stream cipher (symmetric). Input/key/output each have a selectable format.
use super::prelude::*;

fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut s: Vec<u8> = (0..=255).collect();
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + s[i] as usize + key[i % key.len()] as usize) & 0xff;
        s.swap(i, j);
    }
    let mut out = Vec::with_capacity(data.len());
    let (mut i, mut j) = (0usize, 0usize);
    for &byte in data {
        i = (i + 1) & 0xff;
        j = (j + s[i] as usize) & 0xff;
        s.swap(i, j);
        let k = s[(s[i] as usize + s[j] as usize) & 0xff];
        out.push(byte ^ k);
    }
    out
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let key = parse_bytes(pstr(params, "key", ""), pstr(params, "keyFormat", "UTF8"))?;
        if key.is_empty() {
            return Err(CoreError::Parse("RC4 需要密钥".into()));
        }
        let data = parse_bytes(in_text(inputs, "text")?, pstr(params, "inputFormat", "UTF8"))?;
        let out = rc4(&key, &data);
        let text = format_bytes(&out, pstr(params, "outputFormat", "Hex"));
        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(text));
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(out.into_boxed_slice())),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "rc4",
            CRYPTO,
            "RC4",
            ROSE,
            vec![req("text", "输入", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::text("key", "密钥", "", false),
                ParamSpec::select("keyFormat", "密钥格式", &["UTF8", "Hex", "Base64"], "UTF8"),
                ParamSpec::select("inputFormat", "输入格式", &["UTF8", "Hex", "Base64"], "UTF8"),
                ParamSpec::select("outputFormat", "输出格式", &["Hex", "UTF8", "Base64"], "Hex"),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
