//! Pixel values ↔ image. Dump an image's raw pixel values as text (per channel,
//! decimal or hex), and rebuild an image from a list of numbers. Handy when a
//! challenge hands you a wall of numbers that is really an image, or vice versa.
use image::{Rgba, RgbaImage};

use super::image_util::*;
use super::prelude::*;

// ---------------------------------------------------------- extract pixels
struct Extract;
impl Node for Extract {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let (w, h) = img.dimensions();
        let ch = pstr(p, "channel", "灰度");
        let hex = pstr(p, "base", "十进制") == "十六进制";
        let rows = pbool(p, "rows", true);
        let sep = if pstr(p, "sep", "空格") == "逗号" { "," } else { " " };

        let fmt = |v: u8| if hex { format!("{v:02x}") } else { v.to_string() };
        let row_strs: Vec<String> = (0..h)
            .map(|y| {
                let mut toks: Vec<String> = Vec::with_capacity(w as usize);
                for x in 0..w {
                    let px = img.get_pixel(x, y).0;
                    match ch {
                        "R" => toks.push(fmt(px[0])),
                        "G" => toks.push(fmt(px[1])),
                        "B" => toks.push(fmt(px[2])),
                        "A" => toks.push(fmt(px[3])),
                        "RGB" => toks.extend([fmt(px[0]), fmt(px[1]), fmt(px[2])]),
                        "RGBA" => toks.extend([fmt(px[0]), fmt(px[1]), fmt(px[2]), fmt(px[3])]),
                        _ => toks.push(fmt(luma(px[0], px[1], px[2]))),
                    }
                }
                toks.join(sep)
            })
            .collect();
        let text = if rows { row_strs.join("\n") } else { row_strs.join(sep) };

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert("width".into(), PortValue::Number(w as f64));
        m.insert("height".into(), PortValue::Number(h as f64));
        Ok(m)
    }
}

// ------------------------------------------------------- values → image
struct FromValues;
impl Node for FromValues {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(i, "text")?;
        let radix = if pstr(p, "base", "十进制") == "十六进制" { 16 } else { 10 };
        let channels: usize = match pstr(p, "channels", "灰度(1)") {
            "RGB(3)" => 3,
            "RGBA(4)" => 4,
            _ => 1,
        };

        let mut vals: Vec<u8> = Vec::new();
        for tok in input.split(|c: char| !c.is_ascii_alphanumeric()) {
            let t = tok.trim_start_matches("0x").trim_start_matches("0X");
            if t.is_empty() {
                continue;
            }
            if let Ok(v) = u32::from_str_radix(t, radix) {
                vals.push(v.min(255) as u8);
            }
        }
        let px_count = vals.len() / channels;
        if px_count == 0 {
            return Err(CoreError::Parse("未找到足够的像素数值。".into()));
        }
        let mut w = pnum(p, "width", 0.0) as usize;
        if w == 0 {
            w = (px_count as f64).sqrt().ceil() as usize;
        }
        let w = w.max(1);
        let h = px_count.div_ceil(w);

        let mut img = RgbaImage::new(w as u32, h as u32);
        for idx in 0..px_count {
            let base = idx * channels;
            let px = match channels {
                3 => Rgba([vals[base], vals[base + 1], vals[base + 2], 255]),
                4 => Rgba([vals[base], vals[base + 1], vals[base + 2], vals[base + 3]]),
                _ => {
                    let v = vals[base];
                    Rgba([v, v, v, 255])
                }
            };
            img.put_pixel((idx % w) as u32, (idx / w) as u32, px);
        }
        image_out(&img)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "pixel_extract",
            IMG,
            "提取像素值",
            CYAN,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "数值", PortType::Text),
                opt("width", "宽", PortType::Number),
                opt("height", "高", PortType::Number),
            ],
            vec![
                ParamSpec::select("channel", "通道", &["灰度", "R", "G", "B", "A", "RGB", "RGBA"], "灰度"),
                ParamSpec::select("base", "进制", &["十进制", "十六进制"], "十进制"),
                ParamSpec::select("sep", "分隔符", &["空格", "逗号"], "空格"),
                ParamSpec::toggle("rows", "按行换行", true),
            ],
        ),
        Arc::new(|| Arc::new(Extract)),
    );
    reg.register(
        desc(
            "values_to_image",
            IMG,
            "像素值转图像",
            CYAN,
            vec![req("text", "数值", PortType::Text)],
            vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)],
            vec![
                ParamSpec::select("channels", "通道数", &["灰度(1)", "RGB(3)", "RGBA(4)"], "灰度(1)"),
                ParamSpec::number("width", "宽度(0=自动)", 0.0, 100000.0, 1.0, 0.0),
                ParamSpec::select("base", "进制", &["十进制", "十六进制"], "十进制"),
            ],
        ),
        Arc::new(|| Arc::new(FromValues)),
    );
}
