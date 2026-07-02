//! Channel & bit-plane image nodes: extract / split / merge / swap channels and
//! isolate a single bit-plane (a staple of image steganography).
use image::{Rgba, RgbaImage};

use super::image_util::*;
use super::prelude::*;

fn img_in() -> PortSpec {
    req("data", "图片", PortType::Any)
}
fn img_out() -> Vec<PortSpec> {
    vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)]
}
fn chan_index(c: char) -> usize {
    match c.to_ascii_uppercase() {
        'G' => 1,
        'B' => 2,
        'A' => 3,
        _ => 0,
    }
}
fn luma_px(px: &Rgba<u8>) -> u8 {
    let [r, g, b, _] = px.0;
    luma(r, g, b)
}

struct Extract;
impl Node for Extract {
    fn run(&self, inputs: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(inputs, "data")?;
        let ch = pstr(p, "channel", "R");
        let gray = pstr(p, "output", "灰度图") == "灰度图" || ch == "灰度";
        let mut out = RgbaImage::new(img.width(), img.height());
        for (x, y, px) in img.enumerate_pixels() {
            let [r, g, b, a] = px.0;
            let v = match ch {
                "G" => g,
                "B" => b,
                "A" => a,
                "灰度" => luma(r, g, b),
                _ => r,
            };
            let np = if gray {
                Rgba([v, v, v, 255])
            } else {
                match ch {
                    "G" => Rgba([0, v, 0, 255]),
                    "B" => Rgba([0, 0, v, 255]),
                    "A" => Rgba([v, v, v, v]),
                    _ => Rgba([v, 0, 0, 255]),
                }
            };
            out.put_pixel(x, y, np);
        }
        image_out(&out)
    }
}

struct Split;
impl Node for Split {
    fn run(&self, inputs: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(inputs, "data")?;
        let (w, h) = img.dimensions();
        let mut planes = [RgbaImage::new(w, h), RgbaImage::new(w, h), RgbaImage::new(w, h), RgbaImage::new(w, h)];
        for (x, y, px) in img.enumerate_pixels() {
            for (c, plane) in planes.iter_mut().enumerate() {
                let v = px.0[c];
                plane.put_pixel(x, y, Rgba([v, v, v, 255]));
            }
        }
        let mut m = PortMap::new();
        for (name, plane) in [("r", &planes[0]), ("g", &planes[1]), ("b", &planes[2]), ("a", &planes[3])] {
            m.insert(name.to_string(), PortValue::Image(data_url(&to_png(plane)?, "image/png")));
        }
        Ok(m)
    }
}

struct Merge;
impl Node for Merge {
    fn run(&self, inputs: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let r = load_image(inputs, "r")?;
        let g = load_image(inputs, "g")?;
        let b = load_image(inputs, "b")?;
        let a = load_image(inputs, "a").ok();
        let mut w = r.width().min(g.width()).min(b.width());
        let mut h = r.height().min(g.height()).min(b.height());
        if let Some(ai) = &a {
            w = w.min(ai.width());
            h = h.min(ai.height());
        }
        let mut out = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let av = a.as_ref().map(|ai| luma_px(ai.get_pixel(x, y))).unwrap_or(255);
                out.put_pixel(x, y, Rgba([luma_px(r.get_pixel(x, y)), luma_px(g.get_pixel(x, y)), luma_px(b.get_pixel(x, y)), av]));
            }
        }
        image_out(&out)
    }
}

struct Swap;
impl Node for Swap {
    fn run(&self, inputs: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(inputs, "data")?;
        let order: Vec<char> = pstr(p, "order", "RGB").chars().collect();
        let (oi, gi, bi) = (
            chan_index(*order.first().unwrap_or(&'R')),
            chan_index(*order.get(1).unwrap_or(&'G')),
            chan_index(*order.get(2).unwrap_or(&'B')),
        );
        let mut out = RgbaImage::new(img.width(), img.height());
        for (x, y, px) in img.enumerate_pixels() {
            let c = px.0;
            out.put_pixel(x, y, Rgba([c[oi], c[gi], c[bi], c[3]]));
        }
        image_out(&out)
    }
}

struct BitPlane;
impl Node for BitPlane {
    fn run(&self, inputs: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(inputs, "data")?;
        let ch = pstr(p, "channel", "R");
        let bit = pnum(p, "bit", 0.0).clamp(0.0, 7.0) as u8;
        let mut out = RgbaImage::new(img.width(), img.height());
        for (x, y, px) in img.enumerate_pixels() {
            let [r, g, b, a] = px.0;
            let v = match ch {
                "G" => g,
                "B" => b,
                "A" => a,
                "灰度" => luma(r, g, b),
                _ => r,
            };
            let bv = if (v >> bit) & 1 == 1 { 255 } else { 0 };
            out.put_pixel(x, y, Rgba([bv, bv, bv, 255]));
        }
        image_out(&out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let chans = &["R", "G", "B", "A", "灰度"];
    reg.register(
        desc(
            "channel_extract",
            IMG,
            "通道提取",
            FUCHSIA,
            vec![img_in()],
            img_out(),
            vec![
                ParamSpec::select("channel", "通道", chans, "R"),
                ParamSpec::select("output", "输出", &["灰度图", "仅该通道"], "灰度图"),
            ],
        ),
        Arc::new(|| Arc::new(Extract)),
    );
    reg.register(
        desc(
            "channel_split",
            IMG,
            "通道分离",
            FUCHSIA,
            vec![img_in()],
            vec![
                req("r", "R", PortType::Image),
                req("g", "G", PortType::Image),
                req("b", "B", PortType::Image),
                opt("a", "A", PortType::Image),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(Split)),
    );
    reg.register(
        desc(
            "channel_merge",
            IMG,
            "通道合并",
            FUCHSIA,
            vec![
                req("r", "R (灰度)", PortType::Any),
                req("g", "G (灰度)", PortType::Any),
                req("b", "B (灰度)", PortType::Any),
                opt("a", "A (灰度)", PortType::Any),
            ],
            img_out(),
            vec![],
        ),
        Arc::new(|| Arc::new(Merge)),
    );
    reg.register(
        desc(
            "channel_swap",
            IMG,
            "通道交换",
            FUCHSIA,
            vec![img_in()],
            img_out(),
            vec![ParamSpec::select("order", "顺序", &["RGB", "RBG", "GRB", "GBR", "BRG", "BGR"], "BGR")],
        ),
        Arc::new(|| Arc::new(Swap)),
    );
    reg.register(
        desc(
            "bit_plane",
            IMG,
            "位平面提取",
            FUCHSIA,
            vec![img_in()],
            img_out(),
            vec![
                ParamSpec::select("channel", "通道", chans, "R"),
                ParamSpec::number("bit", "位 (0=最低)", 0.0, 7.0, 1.0, 0.0),
            ],
        ),
        Arc::new(|| Arc::new(BitPlane)),
    );
}
