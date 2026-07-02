//! imageIN（hinaLayer）变深度 LSB 文件隐写节点测试。
//!
//! 两类保证：
//! 1. **真实样本对照** —— `fixtures/imagein_rr_INfile1.png` 由真实工具生成，解码须
//!    还原出文件名 `100.txt` 与其 12690 字节内容（深度 2）。这是对位序/通道序/深度
//!    标记/容器格式的独立交叉验证。
//! 2. **往返一致** —— 自建载体图 → 嵌入 → 提取，跨通道与深度均能原样还原（含载荷
//!    里混入 `FE FF` 也不影响容器切分）。

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

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
    img.write_to(&mut Cursor::new(&mut b), ImageFormat::Png)
        .unwrap();
    b
}

fn run(id: &str, ports: &[(&str, PortValue)], params: serde_json::Value) -> PortMap {
    let reg = default_registry();
    let mut m = HashMap::new();
    for (k, v) in ports {
        m.insert(k.to_string(), v.clone());
    }
    GraphExecutor::run_node(&reg, id, &m, &params, &NullSink, &CancellationToken::new()).unwrap()
}

fn bytes_of(m: &PortMap, port: &str) -> Vec<u8> {
    match m.get(port) {
        Some(PortValue::Bytes(b)) => b.to_vec(),
        other => panic!("expected Bytes at '{port}', got {other:?}"),
    }
}
fn text_of(m: &PortMap, port: &str) -> String {
    match m.get(port) {
        Some(PortValue::Text(s)) => s.clone(),
        other => panic!("expected Text at '{port}', got {other:?}"),
    }
}

/// A deterministic non-uniform cover image (varied low bits per channel).
fn cover(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        Rgba([
            (x.wrapping_mul(7).wrapping_add(y) & 0xff) as u8,
            (y.wrapping_mul(13).wrapping_add(x * 2) & 0xff) as u8,
            (x ^ y).wrapping_mul(5) as u8,
            255,
        ])
    })
}

// -------------------------------------------------- 1. real-tool conformance
#[test]
fn extract_real_imagein_sample() {
    let sample = include_bytes!("fixtures/imagein_rr_INfile1.png").to_vec();
    let out = run(
        "imagein_extract",
        &[(
            "data",
            PortValue::Bytes(Arc::from(sample.into_boxed_slice())),
        )],
        json!({}), // 全部(BGR) + 深度自动
    );

    assert_eq!(text_of(&out, "filename"), "100.txt");
    let data = bytes_of(&out, "bytes");
    assert_eq!(data.len(), 12690, "embedded file size");
    // 载荷是 GBK 文本 "test 测试 1…"，以 '=' 结尾。
    assert!(data.starts_with(b"test \xb2\xe2\xca\xd4 1"), "GBK 前缀");
    assert_eq!(*data.last().unwrap(), b'=');
    // 控制台变深度格式：自动识别为 深度2 + 跳过 (0,0)。
    let report = text_of(&out, "report");
    assert!(
        report.contains("深度=2") && report.contains("跳过(0,0)"),
        "{report}"
    );
}

/// GUI 版排布：从 (0,0) 起、无深度标记、深度 1、BGR、字节高位在前。用独立于本节点
/// embed 的手工写入器构造，并验证 **GBK 文件名** 正确还原。对应用户实测的
/// `个人账单.xlsx` 那类真实图（不便入库，故以合成图锁定该解码路径）。
#[test]
fn extract_gui_format_no_skip_depth1_gbk_name() {
    let payload = b"PK\x03\x04 pretend zip body \x00\x01\xfe\xff flag{imageIN_gui}";
    let mut container = vec![0xFFu8, 0xFE];
    container.extend_from_slice(payload);
    container.extend_from_slice(&[0xFE, 0xFF]);
    container.extend_from_slice(&[0xb8, 0xf6, 0xc8, 0xcb]); // GBK “个人”
    container.extend_from_slice(b".zip");
    container.extend_from_slice(&[0xFE, 0xFF, 0x98, 0x0A, 0x14, 0xFD, 0xFE, 0xFF]);

    // 手工写入：行主序、含 (0,0)、每像素 B,G,R 各写 1 个 bit0，源字节高位在前。
    let mut img = cover(80, 80);
    let bits: Vec<u8> = container
        .iter()
        .flat_map(|&b| (0..8).rev().map(move |k| (b >> k) & 1))
        .collect();
    let (w, h) = img.dimensions();
    let mut bi = 0usize;
    'o: for y in 0..h {
        for x in 0..w {
            let px = img.get_pixel_mut(x, y);
            for &idx in &[2usize, 1, 0] {
                // OpenCV BGR → rgba 下标 B=2,G=1,R=0
                if bi >= bits.len() {
                    break 'o;
                }
                px.0[idx] = (px.0[idx] & !1) | bits[bi];
                bi += 1;
            }
        }
    }

    let out = run(
        "imagein_extract",
        &[(
            "data",
            PortValue::Bytes(Arc::from(png(&img).into_boxed_slice())),
        )],
        json!({}),
    );
    assert_eq!(text_of(&out, "filename"), "个人.zip");
    assert_eq!(bytes_of(&out, "bytes"), payload);
    let report = text_of(&out, "report");
    assert!(
        report.contains("深度=1") && report.contains("含(0,0)"),
        "{report}"
    );
}

// -------------------------------------------------- 2. round-trips
/// 嵌入再提取，断言文件名与字节完全一致。
fn roundtrip(payload: &[u8], filename: &str, channels: &str, depth: f64) {
    let embedded = run(
        "imagein_embed",
        &[
            (
                "data",
                PortValue::Bytes(Arc::from(png(&cover(64, 64)).into_boxed_slice())),
            ),
            (
                "file",
                PortValue::Bytes(Arc::from(payload.to_vec().into_boxed_slice())),
            ),
        ],
        json!({ "filename": filename, "channels": channels, "depth": depth }),
    );
    let stego = bytes_of(&embedded, "bytes");

    let out = run(
        "imagein_extract",
        &[(
            "data",
            PortValue::Bytes(Arc::from(stego.into_boxed_slice())),
        )],
        // 深度用自动，验证嵌入写下的深度标记能被读回。
        json!({ "channels": channels, "depth": 0.0 }),
    );
    assert_eq!(
        text_of(&out, "filename"),
        filename,
        "channels={channels} depth={depth}"
    );
    assert_eq!(
        bytes_of(&out, "bytes"),
        payload,
        "channels={channels} depth={depth}"
    );
}

#[test]
fn roundtrip_all_channels() {
    // 载荷里故意混入 FE FF / FF FE / NUL / 高位字节，考验容器切分的 rfind。
    let payload = b"\x00\x01\xfe\xff mid \xfe\xff flag{imageIN_\xff\x00_roundtrip} end";
    for ch in ["全部(BGR)", "B(蓝)", "G(绿)", "R(红)"] {
        roundtrip(payload, "flag.bin", ch, 0.0);
    }
}

#[test]
fn roundtrip_forced_depth() {
    let payload = b"secret payload for a forced depth run";
    for d in [2.0, 3.0, 5.0, 8.0] {
        roundtrip(payload, "note.txt", "全部(BGR)", d);
    }
}

#[test]
fn roundtrip_text_payload() {
    // `file` 端口接文本时按 UTF-8 字节处理。
    let out = run(
        "imagein_embed",
        &[
            (
                "data",
                PortValue::Bytes(Arc::from(png(&cover(48, 48)).into_boxed_slice())),
            ),
            ("file", PortValue::Text("flag{文本载荷}".into())),
        ],
        json!({ "filename": "f.txt" }),
    );
    let stego = bytes_of(&out, "bytes");
    let got = run(
        "imagein_extract",
        &[(
            "data",
            PortValue::Bytes(Arc::from(stego.into_boxed_slice())),
        )],
        json!({}),
    );
    assert_eq!(text_of(&got, "text"), "flag{文本载荷}");
    assert_eq!(text_of(&got, "filename"), "f.txt");
}

// -------------------------------------------------- 3. non-imageIN input
#[test]
fn plain_image_reports_absent() {
    let out = run(
        "imagein_extract",
        &[(
            "data",
            PortValue::Bytes(Arc::from(png(&cover(16, 16)).into_boxed_slice())),
        )],
        json!({}),
    );
    assert_eq!(text_of(&out, "filename"), "");
    assert!(text_of(&out, "report").contains("未发现"));
    // 仍返回原始比特流以便手动分析（非空）。
    assert!(!bytes_of(&out, "bytes").is_empty());
}

// -------------------------------------------------- 4. capacity guard
#[test]
fn embed_rejects_oversized_payload() {
    let reg = default_registry();
    let mut m = HashMap::new();
    m.insert(
        "data".to_string(),
        PortValue::Bytes(Arc::from(png(&cover(8, 8)).into_boxed_slice())),
    );
    m.insert(
        "file".to_string(),
        PortValue::Bytes(Arc::from(vec![0u8; 4096].into_boxed_slice())),
    );
    let err = GraphExecutor::run_node(
        &reg,
        "imagein_embed",
        &m,
        &json!({ "depth": 2.0 }),
        &NullSink,
        &CancellationToken::new(),
    )
    .unwrap_err();
    assert!(format!("{err}").contains("容量不足"));
}
