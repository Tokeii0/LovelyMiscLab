//! Per-pixel & neighbourhood image filters: grayscale, invert, threshold(+Otsu),
//! brightness/contrast, gamma, histogram equalization, Sobel edges, constant XOR.
use image::{Rgba, RgbaImage};

use super::image_util::*;
use super::prelude::*;

fn map_rgb(img: &RgbaImage, mut f: impl FnMut(u8, u8, u8, u8) -> [u8; 4]) -> RgbaImage {
    let mut out = RgbaImage::new(img.width(), img.height());
    for (x, y, px) in img.enumerate_pixels() {
        let [r, g, b, a] = px.0;
        out.put_pixel(x, y, Rgba(f(r, g, b, a)));
    }
    out
}

struct Gray;
impl Node for Gray {
    fn run(&self, i: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        image_out(&map_rgb(&load_image(i, "data")?, |r, g, b, a| {
            let v = luma(r, g, b);
            [v, v, v, a]
        }))
    }
}

struct Invert;
impl Node for Invert {
    fn run(&self, i: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        image_out(&map_rgb(&load_image(i, "data")?, |r, g, b, a| [255 - r, 255 - g, 255 - b, a]))
    }
}

struct Threshold;
impl Node for Threshold {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let thr = if pbool(p, "auto", false) {
            let mut hist = [0u32; 256];
            for px in img.pixels() {
                hist[luma(px.0[0], px.0[1], px.0[2]) as usize] += 1;
            }
            otsu(&hist, img.width() * img.height())
        } else {
            pnum(p, "threshold", 128.0).clamp(0.0, 255.0) as u8
        };
        let inv = pbool(p, "invert", false);
        image_out(&map_rgb(&img, |r, g, b, _| {
            let on = (luma(r, g, b) > thr) ^ inv;
            let v = if on { 255 } else { 0 };
            [v, v, v, 255]
        }))
    }
}

struct BrightnessContrast;
impl Node for BrightnessContrast {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let bright = pnum(p, "brightness", 0.0) as f32;
        let factor = (pnum(p, "contrast", 0.0) as f32 + 100.0) / 100.0; // 0..2
        let adj = |v: u8| ((v as f32 - 128.0) * factor + 128.0 + bright).round().clamp(0.0, 255.0) as u8;
        image_out(&map_rgb(&load_image(i, "data")?, |r, g, b, a| [adj(r), adj(g), adj(b), a]))
    }
}

struct Gamma;
impl Node for Gamma {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let g = pnum(p, "gamma", 1.0).clamp(0.1, 5.0) as f32;
        let lut: Vec<u8> =
            (0..256).map(|v| (255.0 * (v as f32 / 255.0).powf(g)).round().clamp(0.0, 255.0) as u8).collect();
        image_out(&map_rgb(&load_image(i, "data")?, |r, gr, b, a| {
            [lut[r as usize], lut[gr as usize], lut[b as usize], a]
        }))
    }
}

struct HistEqualize;
impl Node for HistEqualize {
    fn run(&self, i: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let total = (img.width() * img.height()) as f64;
        // Independent per-channel equalization.
        let mut lut = [[0u8; 256]; 3];
        for (c, lut_c) in lut.iter_mut().enumerate() {
            let mut hist = [0u32; 256];
            for px in img.pixels() {
                hist[px.0[c] as usize] += 1;
            }
            let mut cdf = 0u32;
            let cdf_min = hist.iter().find(|&&h| h > 0).copied().unwrap_or(0) as f64;
            let denom = (total - cdf_min).max(1.0);
            for v in 0..256 {
                cdf += hist[v];
                lut_c[v] = (((cdf as f64 - cdf_min) / denom) * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
        image_out(&map_rgb(&img, |r, g, b, a| [lut[0][r as usize], lut[1][g as usize], lut[2][b as usize], a]))
    }
}

struct EdgeDetect;
impl Node for EdgeDetect {
    fn run(&self, i: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let (w, h) = img.dimensions();
        let gray: Vec<i32> = img.pixels().map(|p| luma(p.0[0], p.0[1], p.0[2]) as i32).collect();
        let at = |x: i64, y: i64| -> i32 {
            let cx = x.clamp(0, w as i64 - 1) as u32;
            let cy = y.clamp(0, h as i64 - 1) as u32;
            gray[(cy * w + cx) as usize]
        };
        let mut out = RgbaImage::new(w, h);
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                let gx = -at(x - 1, y - 1) - 2 * at(x - 1, y) - at(x - 1, y + 1)
                    + at(x + 1, y - 1) + 2 * at(x + 1, y) + at(x + 1, y + 1);
                let gy = -at(x - 1, y - 1) - 2 * at(x, y - 1) - at(x + 1, y - 1)
                    + at(x - 1, y + 1) + 2 * at(x, y + 1) + at(x + 1, y + 1);
                let mag = ((gx * gx + gy * gy) as f64).sqrt().min(255.0) as u8;
                out.put_pixel(x as u32, y as u32, Rgba([mag, mag, mag, 255]));
            }
        }
        image_out(&out)
    }
}

struct XorConst;
impl Node for XorConst {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let key = pstr(p, "key", "");
        let kb: Vec<u8> = if pstr(p, "keyFormat", "Hex") == "整数" {
            vec![(key.trim().parse::<i64>().unwrap_or(0) & 0xff) as u8]
        } else {
            hex::decode(key.trim().replace([' ', '\n'], "")).unwrap_or_default()
        };
        if kb.is_empty() {
            return Err(CoreError::Parse("异或密钥为空（Hex 或整数）".into()));
        }
        let img = load_image(i, "data")?;
        let mut ki = 0usize;
        let out = map_rgb(&img, |r, g, b, a| {
            let mut ch = [r, g, b];
            for v in ch.iter_mut() {
                *v ^= kb[ki % kb.len()];
                ki += 1;
            }
            [ch[0], ch[1], ch[2], a]
        });
        image_out(&out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let din = || vec![req("data", "图片", PortType::Any)];
    let dout = || vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)];
    reg.register(desc("grayscale", IMG, "灰度化", INDIGO, din(), dout(), vec![]), Arc::new(|| Arc::new(Gray)));
    reg.register(desc("image_invert", IMG, "反色", INDIGO, din(), dout(), vec![]), Arc::new(|| Arc::new(Invert)));
    reg.register(
        desc(
            "threshold",
            IMG,
            "二值化",
            INDIGO,
            din(),
            dout(),
            vec![
                ParamSpec::number("threshold", "阈值", 0.0, 255.0, 1.0, 128.0),
                ParamSpec::toggle("auto", "自动(Otsu)", false),
                ParamSpec::toggle("invert", "反转", false),
            ],
        ),
        Arc::new(|| Arc::new(Threshold)),
    );
    reg.register(
        desc(
            "brightness_contrast",
            IMG,
            "亮度对比度",
            INDIGO,
            din(),
            dout(),
            vec![
                ParamSpec::number("brightness", "亮度", -255.0, 255.0, 1.0, 0.0),
                ParamSpec::number("contrast", "对比度", -100.0, 100.0, 1.0, 0.0),
            ],
        ),
        Arc::new(|| Arc::new(BrightnessContrast)),
    );
    reg.register(
        desc(
            "gamma",
            IMG,
            "伽马校正",
            INDIGO,
            din(),
            dout(),
            vec![ParamSpec::number("gamma", "γ (<1变亮)", 0.1, 5.0, 0.1, 1.0)],
        ),
        Arc::new(|| Arc::new(Gamma)),
    );
    reg.register(desc("hist_equalize", IMG, "直方图均衡", INDIGO, din(), dout(), vec![]), Arc::new(|| Arc::new(HistEqualize)));
    reg.register(desc("edge_detect", IMG, "边缘检测", INDIGO, din(), dout(), vec![]), Arc::new(|| Arc::new(EdgeDetect)));
    reg.register(
        desc(
            "image_xor",
            IMG,
            "常数异或",
            INDIGO,
            din(),
            dout(),
            vec![
                ParamSpec::text("key", "密钥", "ff", false),
                ParamSpec::select("keyFormat", "格式", &["Hex", "整数"], "Hex"),
            ],
        ),
        Arc::new(|| Arc::new(XorConst)),
    );
}
