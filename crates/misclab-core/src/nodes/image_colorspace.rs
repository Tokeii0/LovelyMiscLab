//! Color-space component extraction (HSV / YCbCr) as grayscale — extends the
//! channel family for revealing content hidden in a non-RGB component.
use image::{Rgba, RgbaImage};

use super::image_util::*;
use super::prelude::*;

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (rf, gf, bf) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let d = max - min;
    let mut h = if d == 0.0 {
        0.0
    } else if max == rf {
        60.0 * (((gf - bf) / d).rem_euclid(6.0))
    } else if max == gf {
        60.0 * ((bf - rf) / d + 2.0)
    } else {
        60.0 * ((rf - gf) / d + 4.0)
    };
    if h < 0.0 {
        h += 360.0;
    }
    (h, if max == 0.0 { 0.0 } else { d / max }, max) // H:0-360, S:0-1, V:0-1
}

struct N;
impl Node for N {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let ycbcr = pstr(p, "space", "HSV") == "YCbCr";
        let comp = pstr(p, "component", "分量1(H/Y)");
        let idx = if comp.contains('2') { 1 } else if comp.contains('3') { 2 } else { 0 };
        let mut out = RgbaImage::new(img.width(), img.height());
        for (x, y, px) in img.enumerate_pixels() {
            let [r, g, b, _] = px.0;
            let v = if ycbcr {
                let (rf, gf, bf) = (r as f32, g as f32, b as f32);
                [
                    0.299 * rf + 0.587 * gf + 0.114 * bf,
                    128.0 - 0.168736 * rf - 0.331264 * gf + 0.5 * bf,
                    128.0 + 0.5 * rf - 0.418688 * gf - 0.081312 * bf,
                ][idx]
            } else {
                let (h, s, vv) = rgb_to_hsv(r, g, b);
                [h / 360.0 * 255.0, s * 255.0, vv * 255.0][idx]
            };
            let v = v.round().clamp(0.0, 255.0) as u8;
            out.put_pixel(x, y, Rgba([v, v, v, 255]));
        }
        image_out(&out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "colorspace_extract",
            IMG,
            "色彩空间分量",
            FUCHSIA,
            vec![req("data", "图片", PortType::Any)],
            vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)],
            vec![
                ParamSpec::select("space", "色彩空间", &["HSV", "YCbCr"], "HSV"),
                ParamSpec::select("component", "分量", &["分量1(H/Y)", "分量2(S/Cb)", "分量3(V/Cr)"], "分量1(H/Y)"),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
