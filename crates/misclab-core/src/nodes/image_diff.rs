//! Highlight the differences between two images (changed pixels in red over a
//! dimmed base) — reveals hidden overlays / edited regions.
use image::{Rgba, RgbaImage};

use super::image_util::*;
use super::prelude::*;

struct N;
impl Node for N {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let a = load_image(i, "a")?;
        let b = load_image(i, "b")?;
        let (a, b) = align(a, b, false);
        let thresh = pnum(p, "threshold", 16.0).clamp(0.0, 255.0) as u8;
        let (w, h) = a.dimensions();
        let mut out = RgbaImage::new(w, h);
        let mut diffs = 0u64;
        for y in 0..h {
            for x in 0..w {
                let pa = a.get_pixel(x, y).0;
                let pb = b.get_pixel(x, y).0;
                let d = pa[0].abs_diff(pb[0]).max(pa[1].abs_diff(pb[1])).max(pa[2].abs_diff(pb[2]));
                if d > thresh {
                    diffs += 1;
                    out.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                } else {
                    let g = (luma(pa[0], pa[1], pa[2]) as u16 * 35 / 100) as u8; // dim base
                    out.put_pixel(x, y, Rgba([g, g, g, 255]));
                }
            }
        }
        let mut m = image_out(&out)?;
        m.insert("count".to_string(), PortValue::Number(diffs as f64));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "image_diff",
            IMG,
            "图像差异",
            FUCHSIA,
            vec![req("a", "图片 A", PortType::Any), req("b", "图片 B", PortType::Any)],
            vec![
                req("image", "差异图", PortType::Image),
                opt("bytes", "字节", PortType::Bytes),
                opt("count", "差异像素数", PortType::Number),
            ],
            vec![ParamSpec::number("threshold", "阈值", 0.0, 255.0, 1.0, 16.0)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
