//! Bits ↔ image. Render a 0/1 bitstream as a black-and-white bitmap — a common
//! CTF puzzle where a blob of 0s and 1s is really a picture or a QR code — and
//! the reverse: threshold an image down to a 0/1 grid.
//!
//! Convention: `1` → black, `0` → white (QR-like); flip with 取反. So the pair
//! round-trips: a black pixel becomes `1`, which renders back to black.
use image::{Rgba, RgbaImage};

use super::image_util::*;
use super::prelude::*;

// ---------------------------------------------------------------- 01 → image
struct ToImage;
impl Node for ToImage {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(i, "text")?;
        let invert = pbool(p, "invert", false);
        let scale = (pnum(p, "scale", 1.0) as u32).clamp(1, 64);

        // Each non-empty line becomes a row of 0/1 (other chars ignored).
        let lines: Vec<Vec<u8>> = input
            .lines()
            .map(|l| {
                l.chars()
                    .filter_map(|c| match c {
                        '0' => Some(0u8),
                        '1' => Some(1u8),
                        _ => None,
                    })
                    .collect::<Vec<u8>>()
            })
            .filter(|r| !r.is_empty())
            .collect();

        let by_rows = match pstr(p, "mode", "自动") {
            "按行" => true,
            "按宽度" => false,
            _ => lines.len() >= 2,
        };

        let (w, rows): (usize, Vec<Vec<u8>>) = if by_rows {
            let w = lines.iter().map(|r| r.len()).max().unwrap_or(0);
            (w, lines)
        } else {
            let flat: Vec<u8> = lines.into_iter().flatten().collect();
            let n = flat.len();
            let mut w = pnum(p, "width", 0.0) as usize;
            if w == 0 {
                w = (n as f64).sqrt().ceil() as usize;
            }
            let w = w.max(1);
            (w, flat.chunks(w).map(|c| c.to_vec()).collect())
        };

        if w == 0 || rows.is_empty() {
            return Err(CoreError::Parse("未找到 0/1 数据。".into()));
        }
        let h = rows.len();
        let mut img = RgbaImage::new(w as u32 * scale, h as u32 * scale);
        for (y, row) in rows.iter().enumerate() {
            for x in 0..w {
                let bit = row.get(x).copied().unwrap_or(0);
                // 1 → black by default; invert flips.
                let v: u8 = if (bit == 1) ^ invert { 0 } else { 255 };
                for dy in 0..scale {
                    for dx in 0..scale {
                        img.put_pixel(x as u32 * scale + dx, y as u32 * scale + dy, Rgba([v, v, v, 255]));
                    }
                }
            }
        }
        image_out(&img)
    }
}

// ---------------------------------------------------------------- image → 01
struct ToBits;
impl Node for ToBits {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let (w, h) = img.dimensions();
        let invert = pbool(p, "invert", false);

        let thr = if pbool(p, "otsu", false) {
            let mut hist = [0u32; 256];
            for px in img.pixels() {
                hist[luma(px.0[0], px.0[1], px.0[2]) as usize] += 1;
            }
            otsu(&hist, w * h)
        } else {
            (pnum(p, "threshold", 128.0) as i64).clamp(0, 255) as u8
        };

        let mut s = String::with_capacity(((w + 1) * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let px = img.get_pixel(x, y);
                let dark = luma(px.0[0], px.0[1], px.0[2]) < thr;
                // dark → '1' by default (matches 1 → black in ToImage).
                s.push(if dark ^ invert { '1' } else { '0' });
            }
            s.push('\n');
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(s));
        m.insert("width".into(), PortValue::Number(w as f64));
        m.insert("height".into(), PortValue::Number(h as f64));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "bits_to_image",
            IMG,
            "01 转图像",
            TEAL,
            vec![req("text", "0/1 文本", PortType::Text)],
            vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)],
            vec![
                ParamSpec::select("mode", "布局", &["自动", "按行", "按宽度"], "自动"),
                ParamSpec::number("width", "宽度(按宽度,0=自动)", 0.0, 100000.0, 1.0, 0.0),
                ParamSpec::toggle("invert", "取反(1=白)", false),
                ParamSpec::number("scale", "放大倍数", 1.0, 64.0, 1.0, 1.0),
            ],
        ),
        Arc::new(|| Arc::new(ToImage)),
    );
    reg.register(
        desc(
            "image_to_bits",
            IMG,
            "图像转 01",
            TEAL,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "0/1 文本", PortType::Text),
                opt("width", "宽", PortType::Number),
                opt("height", "高", PortType::Number),
            ],
            vec![
                ParamSpec::number("threshold", "阈值", 0.0, 255.0, 1.0, 128.0),
                ParamSpec::toggle("otsu", "自动阈值(Otsu)", false),
                ParamSpec::toggle("invert", "取反(亮=1)", false),
            ],
        ),
        Arc::new(|| Arc::new(ToBits)),
    );
}
