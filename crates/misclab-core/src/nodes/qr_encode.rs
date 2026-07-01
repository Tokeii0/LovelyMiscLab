use std::io::Cursor;

use super::prelude::*;
use base64::Engine as _;

/// Encode text into a QR code, output as an inline image (data URL).
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let scale = params
            .get("scale")
            .and_then(|v| v.as_f64())
            .unwrap_or(8.0)
            .clamp(1.0, 32.0) as u32;

        let code = qrcode::QrCode::new(text.as_bytes())
            .map_err(|e| CoreError::Other(format!("生成二维码失败: {e}")))?;
        let modules = code.width();
        let colors = code.to_colors();
        let quiet = 4u32;
        let dim = (modules as u32 + quiet * 2) * scale;

        let mut img = image::GrayImage::from_pixel(dim, dim, image::Luma([255u8]));
        for y in 0..modules {
            for x in 0..modules {
                if colors[y * modules + x] == qrcode::Color::Dark {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = (quiet + x as u32) * scale + dx;
                            let py = (quiet + y as u32) * scale + dy;
                            img.put_pixel(px, py, image::Luma([0u8]));
                        }
                    }
                }
            }
        }

        let mut png: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|e| CoreError::Other(format!("PNG 编码失败: {e}")))?;
        let data_url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        );
        let mut out = PortMap::new();
        out.insert("image".to_string(), PortValue::Image(data_url));
        out.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(png.into_boxed_slice())),
        );
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "qr_encode",
            ENC,
            "二维码编码",
            TEAL,
            vec![t_in()],
            vec![
                req("image", "二维码", PortType::Image),
                opt("bytes", "PNG字节", PortType::Bytes),
            ],
            vec![ParamSpec::number("scale", "像素倍率", 1.0, 32.0, 1.0, 8.0)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
