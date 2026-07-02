//! Image input: load an image (as a `data:` URL from the picker, or a local path)
//! and expose its raw bytes, an image URL for display, and the data URL as text.
use base64::Engine as _;

use super::prelude::*;

fn mime_for(path: &str) -> &'static str {
    let p = path.to_ascii_lowercase();
    if p.ends_with(".jpg") || p.ends_with(".jpeg") {
        "image/jpeg"
    } else if p.ends_with(".gif") {
        "image/gif"
    } else if p.ends_with(".webp") {
        "image/webp"
    } else if p.ends_with(".bmp") {
        "image/bmp"
    } else {
        "image/png"
    }
}

/// Returns `(display_url, raw_bytes)`.
fn load(input: &str) -> Result<(String, Vec<u8>), CoreError> {
    let s = input.trim();
    if s.is_empty() {
        return Err(CoreError::Other("未选择图片".into()));
    }
    if s.starts_with("data:") {
        let comma = s.find(',').ok_or_else(|| CoreError::Other("无效的 data URL".into()))?;
        let payload = &s[comma + 1..];
        let bytes = if s[..comma].contains(";base64") {
            base64::engine::general_purpose::STANDARD
                .decode(payload.trim())
                .map_err(|e| CoreError::Other(format!("图片 base64 解码失败: {e}")))?
        } else {
            payload.as_bytes().to_vec()
        };
        return Ok((s.to_string(), bytes));
    }
    if s.starts_with("http://") || s.starts_with("https://") {
        // Remote URL — usable for display/vision; bytes unavailable offline.
        return Ok((s.to_string(), Vec::new()));
    }
    let bytes = std::fs::read(s).map_err(|e| CoreError::Other(format!("读取图片失败: {e}")))?;
    let url = format!("data:{};base64,{}", mime_for(s), base64::engine::general_purpose::STANDARD.encode(&bytes));
    Ok((url, bytes))
}

struct N;
impl Node for N {
    fn run(&self, _inputs: &PortMap, params: &serde_json::Value, _ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let (url, bytes) = load(pstr(params, "image", ""))?;
        let mut out = PortMap::new();
        // `bytes` first so the node's output preview isn't a duplicate of the inline
        // param thumbnail (the on-node image comes from the image-picker widget).
        out.insert("bytes".to_string(), PortValue::Bytes(Arc::from(bytes.into_boxed_slice())));
        out.insert("image".to_string(), PortValue::Image(url.clone()));
        out.insert("dataUrl".to_string(), PortValue::Text(url));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "image_input",
            IO,
            "图片输入",
            SLATE,
            vec![],
            vec![
                req("bytes", "字节", PortType::Bytes),
                opt("image", "图片", PortType::Image),
                opt("dataUrl", "数据URL", PortType::Text),
            ],
            vec![ParamSpec::image("image", "图片")],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
