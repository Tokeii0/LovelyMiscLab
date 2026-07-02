//! Shared helpers for the image-processing nodes: load an image from any input
//! representation, encode a result back to a PNG data URL + bytes, align two
//! images, and small pixel utilities.
use std::io::Cursor;

use base64::Engine as _;
use image::RgbaImage;

use super::prelude::*;

fn bytes_from_str(s: &str) -> Result<Vec<u8>, CoreError> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("data:") {
        let comma = rest.find(',').ok_or_else(|| CoreError::Parse("无效的 data URL".into()))?;
        let payload = &rest[comma + 1..];
        if rest[..comma].contains(";base64") {
            return base64::engine::general_purpose::STANDARD
                .decode(payload.trim())
                .map_err(|e| CoreError::Parse(format!("图片 base64 解码失败: {e}")));
        }
        return Ok(payload.as_bytes().to_vec());
    }
    std::fs::read(s).map_err(|e| CoreError::Other(format!("读取图片失败: {e}")))
}

fn source_bytes(v: Option<&PortValue>, port: &str) -> Result<Vec<u8>, CoreError> {
    match v {
        Some(PortValue::Bytes(b)) => Ok(b.to_vec()),
        Some(PortValue::Image(s)) | Some(PortValue::Text(s)) => bytes_from_str(s),
        Some(PortValue::None) | None => Err(CoreError::MissingInput(port.to_string())),
        Some(other) => Err(CoreError::Type(format!("端口 {port} 不是图片: {:?}", other.port_type()))),
    }
}

/// The raw image bytes on a port (before decoding).
pub fn input_bytes(inputs: &PortMap, port: &str) -> Result<Vec<u8>, CoreError> {
    source_bytes(inputs.get(port), port)
}

/// Load an input (Bytes / image data-URL / path) as an RGBA8 image.
pub fn load_image(inputs: &PortMap, port: &str) -> Result<RgbaImage, CoreError> {
    let bytes = source_bytes(inputs.get(port), port)?;
    Ok(image::load_from_memory(&bytes)
        .map_err(|e| CoreError::Parse(format!("图片解码失败: {e}")))?
        .to_rgba8())
}

pub fn to_png(img: &RgbaImage) -> Result<Vec<u8>, CoreError> {
    let mut png: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| CoreError::Other(format!("PNG 编码失败: {e}")))?;
    Ok(png)
}

pub fn data_url(png: &[u8], mime: &str) -> String {
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

/// Encode an image and return `{ image: dataURL (first, shows on the node), bytes: png }`.
pub fn image_out(img: &RgbaImage) -> Result<PortMap, CoreError> {
    let png = to_png(img)?;
    let mut m = PortMap::new();
    m.insert("image".to_string(), PortValue::Image(data_url(&png, "image/png")));
    m.insert("bytes".to_string(), PortValue::Bytes(Arc::from(png.into_boxed_slice())));
    Ok(m)
}

pub fn luma(r: u8, g: u8, b: u8) -> u8 {
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round().clamp(0.0, 255.0) as u8
}

/// Otsu threshold from a 256-bin histogram of `total` samples.
pub fn otsu(hist: &[u32; 256], total: u32) -> u8 {
    let sum: f64 = hist.iter().enumerate().map(|(i, &c)| i as f64 * c as f64).sum();
    let (mut sum_b, mut w_b, mut max, mut thr) = (0.0f64, 0u32, -1.0f64, 0u8);
    for (t, &count) in hist.iter().enumerate() {
        w_b += count;
        if w_b == 0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0 {
            break;
        }
        sum_b += t as f64 * count as f64;
        let m_b = sum_b / w_b as f64;
        let m_f = (sum - sum_b) / w_f as f64;
        let between = w_b as f64 * w_f as f64 * (m_b - m_f) * (m_b - m_f);
        if between > max {
            max = between;
            thr = t as u8;
        }
    }
    thr
}

/// Crop `img` to `w × h` from the top-left.
pub fn crop_tl(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    image::imageops::crop_imm(img, 0, 0, w, h).to_image()
}

/// Align two images to a shared size: resize B to A, or crop both to the smaller.
pub fn align(a: RgbaImage, b: RgbaImage, resize_b: bool) -> (RgbaImage, RgbaImage) {
    if a.dimensions() == b.dimensions() {
        return (a, b);
    }
    if resize_b {
        let (w, h) = a.dimensions();
        (a, image::imageops::resize(&b, w, h, image::imageops::FilterType::Nearest))
    } else {
        let w = a.width().min(b.width());
        let h = a.height().min(b.height());
        (crop_tl(&a, w, h), crop_tl(&b, w, h))
    }
}
