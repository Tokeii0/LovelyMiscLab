//! Encode text into a QR code as an inline image (data URL) + PNG bytes.
//! Configurable error-correction level, version, module scale, quiet-zone margin,
//! and foreground/background colors.
use super::image_util::image_out;
use super::prelude::*;
use qrcode::{EcLevel, QrCode, Version};

/// Parse `#RRGGBB` / `RRGGBB` → RGB, falling back to `default` on any error.
fn parse_rgb(s: &str, default: [u8; 3]) -> [u8; 3] {
    let h = s.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let Ok(v) = u32::from_str_radix(h, 16) {
            return [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        }
    }
    default
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let scale = (pnum(params, "scale", 8.0) as u32).clamp(1, 64);
        let margin = (pnum(params, "margin", 4.0) as u32).min(64);
        let ec = match pstr(params, "ec", "M") {
            "L" => EcLevel::L,
            "Q" => EcLevel::Q,
            "H" => EcLevel::H,
            _ => EcLevel::M,
        };
        let version = pnum(params, "version", 0.0) as i16;

        let code = if (1..=40).contains(&version) {
            QrCode::with_version(text.as_bytes(), Version::Normal(version), ec)
        } else {
            QrCode::with_error_correction_level(text.as_bytes(), ec)
        }
        .map_err(|e| {
            CoreError::Other(format!(
                "生成二维码失败: {e}（数据可能超出所选版本/纠错等级容量）"
            ))
        })?;

        let dark = parse_rgb(pstr(params, "dark", "#000000"), [0, 0, 0]);
        let light = parse_rgb(pstr(params, "light", "#ffffff"), [255, 255, 255]);
        let dark_px = image::Rgba([dark[0], dark[1], dark[2], 255]);

        let modules = code.width();
        let colors = code.to_colors();
        let dim = (modules as u32 + margin * 2) * scale;
        let mut img = image::RgbaImage::from_pixel(
            dim,
            dim,
            image::Rgba([light[0], light[1], light[2], 255]),
        );
        for y in 0..modules {
            for x in 0..modules {
                if colors[y * modules + x] == qrcode::Color::Dark {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = (margin + x as u32) * scale + dx;
                            let py = (margin + y as u32) * scale + dy;
                            img.put_pixel(px, py, dark_px);
                        }
                    }
                }
            }
        }
        image_out(&img)
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
            vec![
                ParamSpec::select("ec", "纠错等级", &["L", "M", "Q", "H"], "M"),
                ParamSpec::number("version", "版本(0=自动)", 0.0, 40.0, 1.0, 0.0),
                ParamSpec::number("scale", "像素倍率", 1.0, 64.0, 1.0, 8.0),
                ParamSpec::number("margin", "静默区(模块)", 0.0, 64.0, 1.0, 4.0),
                ParamSpec::text("dark", "前景色", "#000000", false),
                ParamSpec::text("light", "背景色", "#ffffff", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
