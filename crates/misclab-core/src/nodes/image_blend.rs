//! Two-image composition: pixel-math blend (XOR / add / diff / screen / dissolve…)
//! and concatenation.
use image::{Rgba, RgbaImage};

use super::image_util::*;
use super::prelude::*;

const MODES: &[&str] = &[
    "异或", "相加", "相减", "差值", "相乘", "变亮", "变暗", "叠加(alpha混合)", "屏幕", "溶解",
];

fn blend_ch(mode: &str, a: u8, b: u8, alpha: f32) -> u8 {
    match mode {
        "相加" => a.saturating_add(b),
        "相减" => a.saturating_sub(b),
        "差值" => a.abs_diff(b),
        "相乘" => ((a as u16 * b as u16) / 255) as u8,
        "变亮" => a.max(b),
        "变暗" => a.min(b),
        "屏幕" => 255 - (((255 - a) as u16 * (255 - b) as u16) / 255) as u8,
        "叠加(alpha混合)" => (a as f32 * (1.0 - alpha) + b as f32 * alpha).round().clamp(0.0, 255.0) as u8,
        _ => a ^ b, // 异或
    }
}

/// Deterministic per-pixel pseudo-random in [0,1) (for dissolve; reproducible).
fn pseudo(x: u32, y: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(0x9e37_79b9);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
}

struct Blend;
impl Node for Blend {
    fn run(&self, inputs: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let a = load_image(inputs, "a")?;
        let b = load_image(inputs, "b")?;
        let (a, b) = align(a, b, pstr(p, "align", "裁剪到较小") == "缩放B到A");
        let mode = pstr(p, "mode", "异或");
        let alpha = pnum(p, "alpha", 0.5).clamp(0.0, 1.0) as f32;
        let (w, h) = a.dimensions();
        let mut out = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let pa = a.get_pixel(x, y).0;
                let pb = b.get_pixel(x, y).0;
                let np = if mode == "溶解" {
                    if pseudo(x, y) < alpha {
                        Rgba([pb[0], pb[1], pb[2], 255])
                    } else {
                        Rgba([pa[0], pa[1], pa[2], 255])
                    }
                } else {
                    Rgba([
                        blend_ch(mode, pa[0], pb[0], alpha),
                        blend_ch(mode, pa[1], pb[1], alpha),
                        blend_ch(mode, pa[2], pb[2], alpha),
                        255,
                    ])
                };
                out.put_pixel(x, y, np);
            }
        }
        image_out(&out)
    }
}

struct Concat;
impl Node for Concat {
    fn run(&self, inputs: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let a = load_image(inputs, "a")?;
        let b = load_image(inputs, "b")?;
        let horizontal = pstr(p, "direction", "水平") == "水平";
        let (w, h) = if horizontal {
            (a.width() + b.width(), a.height().max(b.height()))
        } else {
            (a.width().max(b.width()), a.height() + b.height())
        };
        let mut out = RgbaImage::new(w, h);
        image::imageops::overlay(&mut out, &a, 0, 0);
        if horizontal {
            image::imageops::overlay(&mut out, &b, a.width() as i64, 0);
        } else {
            image::imageops::overlay(&mut out, &b, 0, a.height() as i64);
        }
        image_out(&out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let two = || vec![req("a", "图片 A", PortType::Any), req("b", "图片 B", PortType::Any)];
    let out = || vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)];
    reg.register(
        desc(
            "image_blend",
            IMG,
            "图像混合",
            FUCHSIA,
            two(),
            out(),
            vec![
                ParamSpec::select("mode", "模式", MODES, "异或"),
                ParamSpec::number("alpha", "alpha (叠加/溶解)", 0.0, 1.0, 0.05, 0.5),
                ParamSpec::select("align", "尺寸对齐", &["裁剪到较小", "缩放B到A"], "裁剪到较小"),
            ],
        ),
        Arc::new(|| Arc::new(Blend)),
    );
    reg.register(
        desc(
            "image_concat",
            IMG,
            "图像拼接",
            FUCHSIA,
            two(),
            out(),
            vec![ParamSpec::select("direction", "方向", &["水平", "垂直"], "水平")],
        ),
        Arc::new(|| Arc::new(Concat)),
    );
}
