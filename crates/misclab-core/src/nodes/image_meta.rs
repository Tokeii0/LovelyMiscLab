//! Image metadata + format conversion.
use super::image_util::*;
use super::prelude::*;

struct Info;
impl Node for Info {
    fn run(&self, i: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let bytes = input_bytes(i, "data")?;
        let fmt = image::guess_format(&bytes).map(|f| format!("{f:?}")).unwrap_or_else(|_| "未知".into());
        let img = image::load_from_memory(&bytes).map_err(|e| CoreError::Parse(format!("图片解码失败: {e}")))?;
        let (w, h) = (img.width(), img.height());
        let text = format!(
            "尺寸: {w}×{h}\n格式: {fmt}\n颜色模式: {:?}\n文件大小: {} 字节",
            img.color(),
            bytes.len()
        );
        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(text));
        m.insert("width".to_string(), PortValue::Number(w as f64));
        m.insert("height".to_string(), PortValue::Number(h as f64));
        Ok(m)
    }
}

struct Convert;
impl Node for Convert {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let dynimg = image::DynamicImage::ImageRgba8(load_image(i, "data")?);
        let (fmt, mime) = match pstr(p, "format", "PNG") {
            "JPEG" => (image::ImageFormat::Jpeg, "image/jpeg"),
            "BMP" => (image::ImageFormat::Bmp, "image/bmp"),
            "GIF" => (image::ImageFormat::Gif, "image/gif"),
            _ => (image::ImageFormat::Png, "image/png"),
        };
        let mut buf: Vec<u8> = Vec::new();
        let res = if matches!(fmt, image::ImageFormat::Jpeg) {
            // JPEG has no alpha channel.
            dynimg.to_rgb8().write_to(&mut std::io::Cursor::new(&mut buf), fmt)
        } else {
            dynimg.write_to(&mut std::io::Cursor::new(&mut buf), fmt)
        };
        res.map_err(|e| CoreError::Other(format!("编码失败: {e}")))?;
        let mut m = PortMap::new();
        m.insert("image".to_string(), PortValue::Image(data_url(&buf, mime)));
        m.insert("bytes".to_string(), PortValue::Bytes(Arc::from(buf.into_boxed_slice())));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "image_info",
            IMG,
            "图像信息",
            TEAL,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "信息", PortType::Text),
                opt("width", "宽", PortType::Number),
                opt("height", "高", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(Info)),
    );
    reg.register(
        desc(
            "image_convert",
            IMG,
            "格式转换",
            TEAL,
            vec![req("data", "图片", PortType::Any)],
            vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)],
            vec![ParamSpec::select("format", "格式", &["PNG", "JPEG", "BMP", "GIF"], "PNG")],
        ),
        Arc::new(|| Arc::new(Convert)),
    );
}
