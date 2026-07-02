//! Image-processing nodes: deterministic pixel/size checks (no external files).

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use base64::Engine as _;
use image::{ImageFormat, Rgba, RgbaImage};
use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::port::PortValue;
use misclab_core::node::PortMap;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::json;

fn png(img: &RgbaImage) -> Vec<u8> {
    let mut b = Vec::new();
    img.write_to(&mut Cursor::new(&mut b), ImageFormat::Png).unwrap();
    b
}
fn solid(w: u32, h: u32, px: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(w, h, Rgba(px))
}
fn run(id: &str, inputs: &[(&str, &RgbaImage)], params: serde_json::Value) -> PortMap {
    let reg = default_registry();
    let mut m = HashMap::new();
    for (k, img) in inputs {
        m.insert(k.to_string(), PortValue::Bytes(Arc::from(png(img).into_boxed_slice())));
    }
    GraphExecutor::run_node(&reg, id, &m, &params, &NullSink, &CancellationToken::new()).unwrap()
}
fn run_raw(id: &str, port: &str, bytes: Vec<u8>, params: serde_json::Value) -> PortMap {
    let reg = default_registry();
    let mut m = HashMap::new();
    m.insert(port.to_string(), PortValue::Bytes(Arc::from(bytes.into_boxed_slice())));
    GraphExecutor::run_node(&reg, id, &m, &params, &NullSink, &CancellationToken::new()).unwrap()
}
fn decoded(m: &PortMap) -> RgbaImage {
    if let Some(PortValue::Bytes(b)) = m.get("bytes") {
        return image::load_from_memory(b).unwrap().to_rgba8();
    }
    if let Some(PortValue::Image(url)) = m.get("image") {
        let b64 = url.split(',').nth(1).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        return image::load_from_memory(&bytes).unwrap().to_rgba8();
    }
    panic!("no image output");
}

#[test]
fn invert_pixel() {
    let m = run("image_invert", &[("data", &solid(1, 1, [10, 20, 30, 255]))], json!({}));
    assert_eq!(decoded(&m).get_pixel(0, 0).0, [245, 235, 225, 255]);
}

#[test]
fn blend_xor() {
    let a = solid(1, 1, [0xF0, 0x0F, 0xAA, 255]);
    let b = solid(1, 1, [0x0F, 0xF0, 0x55, 255]);
    let m = run("image_blend", &[("a", &a), ("b", &b)], json!({ "mode": "异或" }));
    assert_eq!(decoded(&m).get_pixel(0, 0).0, [0xFF, 0xFF, 0xFF, 255]);
}

#[test]
fn channel_extract_r_gray() {
    let m = run("channel_extract", &[("data", &solid(1, 1, [100, 50, 25, 255]))], json!({ "channel": "R", "output": "灰度图" }));
    assert_eq!(decoded(&m).get_pixel(0, 0).0, [100, 100, 100, 255]);
}

#[test]
fn bit_plane_bit0() {
    let on = run("bit_plane", &[("data", &solid(1, 1, [1, 0, 0, 255]))], json!({ "channel": "R", "bit": 0 }));
    assert_eq!(decoded(&on).get_pixel(0, 0).0[0], 255);
    let off = run("bit_plane", &[("data", &solid(1, 1, [2, 0, 0, 255]))], json!({ "channel": "R", "bit": 0 }));
    assert_eq!(decoded(&off).get_pixel(0, 0).0[0], 0);
}

#[test]
fn threshold_128() {
    let hi = run("threshold", &[("data", &solid(1, 1, [200, 200, 200, 255]))], json!({ "threshold": 128 }));
    assert_eq!(decoded(&hi).get_pixel(0, 0).0[0], 255);
    let lo = run("threshold", &[("data", &solid(1, 1, [50, 50, 50, 255]))], json!({ "threshold": 128 }));
    assert_eq!(decoded(&lo).get_pixel(0, 0).0[0], 0);
}

#[test]
fn grayscale_luma() {
    // 0.299*10 + 0.587*20 + 0.114*30 = 18.15 -> 18
    let m = run("grayscale", &[("data", &solid(1, 1, [10, 20, 30, 255]))], json!({}));
    assert_eq!(decoded(&m).get_pixel(0, 0).0, [18, 18, 18, 255]);
}

#[test]
fn xor_const_ff() {
    let m = run("image_xor", &[("data", &solid(1, 1, [10, 20, 30, 255]))], json!({ "key": "ff", "keyFormat": "Hex" }));
    assert_eq!(decoded(&m).get_pixel(0, 0).0, [245, 235, 225, 255]);
}

#[test]
fn channel_swap_bgr() {
    let m = run("channel_swap", &[("data", &solid(1, 1, [10, 20, 30, 255]))], json!({ "order": "BGR" }));
    assert_eq!(decoded(&m).get_pixel(0, 0).0, [30, 20, 10, 255]);
}

#[test]
fn crop_dims() {
    let m = run("image_crop", &[("data", &solid(4, 4, [0, 0, 0, 255]))], json!({ "x": 1, "y": 1, "width": 2, "height": 2 }));
    assert_eq!(decoded(&m).dimensions(), (2, 2));
}

#[test]
fn resize_dims() {
    let m = run("image_resize", &[("data", &solid(4, 4, [0, 0, 0, 255]))], json!({ "width": 2, "height": 2 }));
    assert_eq!(decoded(&m).dimensions(), (2, 2));
}

#[test]
fn concat_horizontal_width() {
    let a = solid(2, 3, [1, 1, 1, 255]);
    let b = solid(3, 3, [2, 2, 2, 255]);
    let m = run("image_concat", &[("a", &a), ("b", &b)], json!({ "direction": "水平" }));
    assert_eq!(decoded(&m).dimensions(), (5, 3));
}

#[test]
fn transform_rotate90_dims() {
    let m = run("image_transform", &[("data", &solid(2, 1, [0, 0, 0, 255]))], json!({ "op": "旋转90°" }));
    assert_eq!(decoded(&m).dimensions(), (1, 2));
}

// ---- advanced ----

#[test]
fn colorspace_ycbcr_y() {
    // Y ≈ luma; (10,20,30) → 18
    let m = run("colorspace_extract", &[("data", &solid(1, 1, [10, 20, 30, 255]))], json!({ "space": "YCbCr", "component": "分量1(H/Y)" }));
    assert_eq!(decoded(&m).get_pixel(0, 0).0[0], 18);
}

#[test]
fn image_diff_count() {
    let a = solid(1, 1, [0, 0, 0, 255]);
    let b = solid(1, 1, [255, 255, 255, 255]);
    let same = run("image_diff", &[("a", &a), ("b", &a)], json!({ "threshold": 16 }));
    assert!(matches!(same.get("count"), Some(PortValue::Number(n)) if *n == 0.0));
    let diff = run("image_diff", &[("a", &a), ("b", &b)], json!({ "threshold": 16 }));
    assert!(matches!(diff.get("count"), Some(PortValue::Number(n)) if *n == 1.0));
    assert_eq!(decoded(&diff).get_pixel(0, 0).0, [255, 0, 0, 255]);
}

#[test]
fn connected_components_two_blobs() {
    let mut img = RgbaImage::new(3, 1);
    img.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
    img.put_pixel(1, 0, Rgba([0, 0, 0, 255]));
    img.put_pixel(2, 0, Rgba([255, 255, 255, 255]));
    let m = run("connected_components", &[("data", &img)], json!({ "threshold": 128 }));
    assert!(matches!(m.get("count"), Some(PortValue::Number(n)) if *n == 2.0));
}

#[test]
fn dft_spectrum_dims() {
    let m = run("dft_spectrum", &[("data", &solid(8, 8, [100, 100, 100, 255]))], json!({}));
    assert_eq!(decoded(&m).dimensions(), (8, 8));
}

#[test]
fn morphology_dilate_grows() {
    let mut img = RgbaImage::new(5, 5);
    img.put_pixel(2, 2, Rgba([255, 255, 255, 255]));
    let m = run("morphology", &[("data", &img)], json!({ "op": "膨胀", "size": 1, "threshold": 128 }));
    let white = decoded(&m).pixels().filter(|p| p.0[0] > 128).count();
    assert!(white > 1, "dilate should grow the white region, got {white}");
}

#[test]
fn template_match_locates() {
    // steep gradient → distinct 2×2 patches; the exact crop at (3,1) matches there.
    let img = RgbaImage::from_fn(6, 6, |x, y| {
        let v = ((x * 40 + y * 7) % 256) as u8;
        Rgba([v, v, v, 255])
    });
    let tmpl = image::imageops::crop_imm(&img, 3, 1, 2, 2).to_image();
    let m = run("template_match", &[("image", &img), ("template", &tmpl)], json!({}));
    match (m.get("x"), m.get("y")) {
        (Some(PortValue::Number(x)), Some(PortValue::Number(y))) => assert_eq!((*x as u32, *y as u32), (3, 1)),
        o => panic!("no location: {o:?}"),
    }
}

#[test]
fn gif_frame_count() {
    let mut buf = Vec::new();
    {
        let mut enc = image::codecs::gif::GifEncoder::new(&mut buf);
        enc.encode_frame(image::Frame::new(solid(4, 4, [200, 100, 50, 255]))).unwrap();
    }
    let m = run_raw("gif_frame", "data", buf, json!({ "index": 0 }));
    assert!(matches!(m.get("count"), Some(PortValue::Number(n)) if *n == 1.0));
    assert_eq!(decoded(&m).dimensions(), (4, 4));
}
