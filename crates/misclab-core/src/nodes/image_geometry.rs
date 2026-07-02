//! Geometric image ops: rotate / flip, crop, resize.
use super::image_util::*;
use super::prelude::*;

struct Transform;
impl Node for Transform {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        use image::imageops;
        let out = match pstr(p, "op", "旋转90°") {
            "旋转180°" => imageops::rotate180(&img),
            "旋转270°" => imageops::rotate270(&img),
            "水平翻转" => imageops::flip_horizontal(&img),
            "垂直翻转" => imageops::flip_vertical(&img),
            _ => imageops::rotate90(&img),
        };
        image_out(&out)
    }
}

struct Crop;
impl Node for Crop {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let (iw, ih) = img.dimensions();
        let x = (pnum(p, "x", 0.0).max(0.0) as u32).min(iw.saturating_sub(1));
        let y = (pnum(p, "y", 0.0).max(0.0) as u32).min(ih.saturating_sub(1));
        let w = (pnum(p, "width", iw as f64).max(1.0) as u32).min(iw - x);
        let h = (pnum(p, "height", ih as f64).max(1.0) as u32).min(ih - y);
        image_out(&image::imageops::crop_imm(&img, x, y, w, h).to_image())
    }
}

struct Resize;
impl Node for Resize {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let (iw, ih) = img.dimensions();
        let w = (pnum(p, "width", iw as f64).max(1.0) as u32).min(10000);
        let mut h = (pnum(p, "height", ih as f64).max(1.0) as u32).min(10000);
        if pbool(p, "keepAspect", false) {
            h = (w as f64 * ih as f64 / iw as f64).round().max(1.0) as u32;
        }
        image_out(&image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let din = || vec![req("data", "图片", PortType::Any)];
    let dout = || vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)];
    reg.register(
        desc(
            "image_transform",
            IMG,
            "旋转翻转",
            TEAL,
            din(),
            dout(),
            vec![ParamSpec::select("op", "操作", &["旋转90°", "旋转180°", "旋转270°", "水平翻转", "垂直翻转"], "旋转90°")],
        ),
        Arc::new(|| Arc::new(Transform)),
    );
    reg.register(
        desc(
            "image_crop",
            IMG,
            "裁剪",
            TEAL,
            din(),
            dout(),
            vec![
                ParamSpec::number("x", "X", 0.0, 100000.0, 1.0, 0.0),
                ParamSpec::number("y", "Y", 0.0, 100000.0, 1.0, 0.0),
                ParamSpec::number("width", "宽", 1.0, 100000.0, 1.0, 100.0),
                ParamSpec::number("height", "高", 1.0, 100000.0, 1.0, 100.0),
            ],
        ),
        Arc::new(|| Arc::new(Crop)),
    );
    reg.register(
        desc(
            "image_resize",
            IMG,
            "缩放",
            TEAL,
            din(),
            dout(),
            vec![
                ParamSpec::number("width", "宽", 1.0, 10000.0, 1.0, 256.0),
                ParamSpec::number("height", "高", 1.0, 10000.0, 1.0, 256.0),
                ParamSpec::toggle("keepAspect", "保持宽高比", false),
            ],
        ),
        Arc::new(|| Arc::new(Resize)),
    );
}
