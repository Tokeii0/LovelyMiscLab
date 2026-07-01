//! Character-set conversion (编码切换) via `encoding_rs`: text ↔ bytes in a chosen
//! charset. Handy for recovering 乱码 (GBK/UTF-8/Big5 confusion).
use super::prelude::*;

const CHARSETS: &[&str] = &[
    "UTF-8",
    "UTF-16LE",
    "UTF-16BE",
    "GBK",
    "GB18030",
    "Big5",
    "Shift-JIS",
    "EUC-JP",
    "EUC-KR",
    "Windows-1252",
    "Windows-1251",
    "ISO-8859-1",
    "KOI8-R",
];

fn label(name: &str) -> &'static str {
    match name {
        "UTF-16LE" => "utf-16le",
        "UTF-16BE" => "utf-16be",
        "GBK" => "gbk",
        "GB18030" => "gb18030",
        "Big5" => "big5",
        "Shift-JIS" => "shift_jis",
        "EUC-JP" => "euc-jp",
        "EUC-KR" => "euc-kr",
        "Windows-1252" => "windows-1252",
        "Windows-1251" => "windows-1251",
        "ISO-8859-1" => "iso-8859-1",
        "KOI8-R" => "koi8-r",
        _ => "utf-8",
    }
}

fn encoding(name: &str) -> &'static encoding_rs::Encoding {
    encoding_rs::Encoding::for_label(label(name).as_bytes()).unwrap_or(encoding_rs::UTF_8)
}

/// Text → bytes in the chosen charset.
struct EncodeText;
impl Node for EncodeText {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let cs = pstr(params, "charset", "UTF-8");
        // encoding_rs cannot *encode* to UTF-16, so handle those directly.
        let bytes: Vec<u8> = match cs {
            "UTF-16LE" => text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect(),
            "UTF-16BE" => text.encode_utf16().flat_map(|u| u.to_be_bytes()).collect(),
            _ => encoding(cs).encode(text).0.into_owned(),
        };
        let mut m = PortMap::new();
        m.insert("hex".to_string(), PortValue::Text(hex::encode(&bytes)));
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
        );
        Ok(m)
    }
}

/// Bytes → text using the chosen charset.
struct DecodeText;
impl Node for DecodeText {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let (text, _, _) = encoding(pstr(params, "charset", "UTF-8")).decode(&data);
        Ok(out_text(text.into_owned()))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "encode_text",
            CHARSET,
            "文本编码",
            TEAL,
            vec![req("text", "文本", PortType::Text)],
            vec![
                req("hex", "hex", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![ParamSpec::select("charset", "字符集", CHARSETS, "UTF-8")],
        ),
        Arc::new(|| Arc::new(EncodeText)),
    );
    reg.register(
        desc(
            "decode_text",
            CHARSET,
            "文本解码",
            TEAL,
            vec![req("data", "字节/文本", PortType::Any)],
            vec![req("text", "文本", PortType::Text)],
            vec![ParamSpec::select("charset", "字符集", CHARSETS, "UTF-8")],
        ),
        Arc::new(|| Arc::new(DecodeText)),
    );
}
