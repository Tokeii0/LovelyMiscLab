//! Advanced analysis via `imageproc`: connected-component labelling, binary
//! morphology (erode/dilate/open/close), and template matching.
use image::{GrayImage, Luma, Rgba, RgbaImage};
use imageproc::distance_transform::Norm;
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::morphology::{close, dilate, erode, open};
use imageproc::rect::Rect;
use imageproc::region_labelling::{connected_components, Connectivity};
use imageproc::template_matching::{find_extremes, match_template, MatchTemplateMethod};

use super::image_util::*;
use super::prelude::*;

fn binarize(img: &RgbaImage, thr: u8) -> GrayImage {
    let g = image::imageops::grayscale(img);
    GrayImage::from_fn(g.width(), g.height(), |x, y| {
        if g.get_pixel(x, y).0[0] > thr { Luma([255]) } else { Luma([0]) }
    })
}
fn gray_to_rgba(g: &GrayImage) -> RgbaImage {
    RgbaImage::from_fn(g.width(), g.height(), |x, y| {
        let v = g.get_pixel(x, y).0[0];
        Rgba([v, v, v, 255])
    })
}
fn label_color(l: u32) -> Rgba<u8> {
    let r = (l.wrapping_mul(2_654_435_761) >> 8) as u8;
    let g = (l.wrapping_mul(40_503) >> 3) as u8;
    let b = (l.wrapping_mul(1_103_515_245).wrapping_add(12_345) >> 12) as u8;
    Rgba([r | 0x40, g | 0x40, b | 0x40, 255])
}

struct Connected;
impl Node for Connected {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let bin = binarize(&load_image(i, "data")?, pnum(p, "threshold", 128.0).clamp(0.0, 255.0) as u8);
        let labels = connected_components(&bin, Connectivity::Eight, Luma([0u8]));
        let count = labels.pixels().map(|px| px.0[0]).max().unwrap_or(0);
        let mut out = RgbaImage::new(labels.width(), labels.height());
        for (x, y, px) in labels.enumerate_pixels() {
            let l = px.0[0];
            out.put_pixel(x, y, if l == 0 { Rgba([0, 0, 0, 255]) } else { label_color(l) });
        }
        let mut m = image_out(&out)?;
        m.insert("count".to_string(), PortValue::Number(count as f64));
        Ok(m)
    }
}

struct Morphology;
impl Node for Morphology {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let bin = binarize(&load_image(i, "data")?, pnum(p, "threshold", 128.0).clamp(0.0, 255.0) as u8);
        let k = pnum(p, "size", 1.0).clamp(0.0, 50.0) as u8;
        let res = match pstr(p, "op", "膨胀") {
            "腐蚀" => erode(&bin, Norm::LInf, k),
            "开运算" => open(&bin, Norm::LInf, k),
            "闭运算" => close(&bin, Norm::LInf, k),
            _ => dilate(&bin, Norm::LInf, k),
        };
        image_out(&gray_to_rgba(&res))
    }
}

struct TemplateMatch;
impl Node for TemplateMatch {
    fn run(&self, i: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let img = load_image(i, "image")?;
        let tmpl = load_image(i, "template")?;
        let ig = image::imageops::grayscale(&img);
        let tg = image::imageops::grayscale(&tmpl);
        if tg.width() > ig.width() || tg.height() > ig.height() {
            return Err(CoreError::Other("模板比图片还大".into()));
        }
        let result = match_template(&ig, &tg, MatchTemplateMethod::CrossCorrelationNormalized);
        let ext = find_extremes(&result);
        let (mx, my) = ext.max_value_location;
        let mut marked = img.clone();
        draw_hollow_rect_mut(
            &mut marked,
            Rect::at(mx as i32, my as i32).of_size(tg.width(), tg.height()),
            Rgba([255, 0, 0, 255]),
        );
        let mut m = image_out(&marked)?;
        m.insert("text".to_string(), PortValue::Text(format!("匹配位置: ({mx}, {my})  相似度: {:.3}", ext.max_value)));
        m.insert("x".to_string(), PortValue::Number(mx as f64));
        m.insert("y".to_string(), PortValue::Number(my as f64));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let dout = || vec![req("image", "图片", PortType::Image), opt("bytes", "字节", PortType::Bytes)];
    reg.register(
        desc(
            "connected_components",
            IMG,
            "连通域标记",
            FUCHSIA,
            vec![req("data", "图片", PortType::Any)],
            vec![req("image", "标记图", PortType::Image), opt("bytes", "字节", PortType::Bytes), opt("count", "区域数", PortType::Number)],
            vec![ParamSpec::number("threshold", "二值阈值", 0.0, 255.0, 1.0, 128.0)],
        ),
        Arc::new(|| Arc::new(Connected)),
    );
    reg.register(
        desc(
            "morphology",
            IMG,
            "形态学",
            INDIGO,
            vec![req("data", "图片", PortType::Any)],
            dout(),
            vec![
                ParamSpec::select("op", "运算", &["膨胀", "腐蚀", "开运算", "闭运算"], "膨胀"),
                ParamSpec::number("size", "核半径", 0.0, 50.0, 1.0, 1.0),
                ParamSpec::number("threshold", "二值阈值", 0.0, 255.0, 1.0, 128.0),
            ],
        ),
        Arc::new(|| Arc::new(Morphology)),
    );
    reg.register(
        desc(
            "template_match",
            IMG,
            "模板匹配",
            TEAL,
            vec![req("image", "图片", PortType::Any), req("template", "模板", PortType::Any)],
            vec![
                req("image", "标记图", PortType::Image),
                opt("text", "结果", PortType::Text),
                opt("x", "X", PortType::Number),
                opt("y", "Y", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(TemplateMatch)),
    );
}
