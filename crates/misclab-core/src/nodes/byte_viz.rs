//! 可视化分析：把字节流画成图，一眼看出结构 / 加密区 / 文本区。三个节点共享一个文件：
//! - `byte_histogram` 字节直方图：256 个字节值的分布（按类别着色）。
//! - `entropy_curve` 熵曲线：滑窗香农熵随偏移变化（定位加密/压缩/内嵌区）。
//! - `byte_map` 字节分布图：每字节一像素、按类别着色（binvis 式），看整体结构。
//!
//! 都输出 `image`（data:URL，直接在节点上显示、可点开放大）+ `bytes`（PNG）。
use image::{Rgba, RgbaImage};

use super::image_util::image_out;
use super::prelude::*;

/// 按字节类别着色：null / 空白 / 可打印 ASCII / 0xFF / 其它高位。
fn byte_color(b: u8) -> [u8; 3] {
    match b {
        0x00 => [40, 42, 54],
        0x09 | 0x0a | 0x0d => [90, 200, 210], // 空白
        0x20..=0x7e => [90, 200, 130],        // 可打印 → 绿
        0xff => [200, 130, 220],
        _ => [230, 150, 80], // 高位/控制 → 橙
    }
}

fn fill_col(img: &mut RgbaImage, x0: u32, x1: u32, y_top: u32, c: [u8; 3]) {
    let (w, h) = img.dimensions();
    for y in y_top..h {
        for x in x0..x1.min(w) {
            img.put_pixel(x, y, Rgba([c[0], c[1], c[2], 255]));
        }
    }
}

/// 一段字节的香农熵（bits/字节，0..8）。
fn shannon(chunk: &[u8]) -> f64 {
    if chunk.is_empty() {
        return 0.0;
    }
    let mut freq = [0u32; 256];
    for &b in chunk {
        freq[b as usize] += 1;
    }
    let len = chunk.len() as f64;
    let mut h = 0.0;
    for &c in &freq {
        if c > 0 {
            let p = c as f64 / len;
            h -= p * p.log2();
        }
    }
    h
}

// ---------------------------------------------------------- 字节直方图
struct Histogram;
impl Node for Histogram {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        if data.is_empty() {
            return Err(CoreError::Parse("输入为空".into()));
        }
        let mut hist = [0u32; 256];
        for &b in &data {
            hist[b as usize] += 1;
        }
        let max = hist.iter().copied().max().unwrap_or(1).max(1);
        let (w, h) = (512u32, 256u32);
        let mut img = RgbaImage::from_pixel(w, h, Rgba([250, 250, 252, 255]));
        for (b, &count) in hist.iter().enumerate() {
            let bar = ((count as f64 / max as f64) * (h as f64 - 2.0)) as u32;
            let x0 = b as u32 * 2;
            fill_col(&mut img, x0, x0 + 2, h - bar, byte_color(b as u8));
        }
        let mut m = image_out(&img)?;
        m.insert("json".into(), PortValue::Json(serde_json::json!(hist.to_vec())));
        Ok(m)
    }
}

// ---------------------------------------------------------- 熵曲线
struct EntropyCurve;
impl Node for EntropyCurve {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        if data.is_empty() {
            return Err(CoreError::Parse("输入为空".into()));
        }
        let window = (pnum(p, "window", 256.0).max(16.0)) as usize;
        let chunks: Vec<f64> = data.chunks(window).map(shannon).collect();
        let n = chunks.len().max(1);
        let (w, h) = (512u32, 200u32);
        let mut img = RgbaImage::from_pixel(w, h, Rgba([20, 22, 30, 255]));
        for x in 0..w {
            let idx = (x as usize * n / w as usize).min(n - 1);
            let e = chunks[idx].clamp(0.0, 8.0);
            let bar = ((e / 8.0) * (h as f64 - 2.0)) as u32;
            let t = e / 8.0;
            let r = (20.0 + t * 230.0) as u8;
            let b = (20.0 + (1.0 - t) * 230.0) as u8;
            fill_col(&mut img, x, x + 1, h - bar, [r, 90, b]);
        }
        let mut m = image_out(&img)?;
        let maxe = chunks.iter().cloned().fold(0.0f64, f64::max);
        let avg = chunks.iter().sum::<f64>() / n as f64;
        m.insert(
            "text".into(),
            PortValue::Text(format!(
                "窗口 {window} 字节 · {n} 段 · 平均熵 {avg:.2} · 峰值 {maxe:.2} bits/字节\n（接近 8 = 加密/压缩，低 = 文本/填充）"
            )),
        );
        m.insert("json".into(), PortValue::Json(serde_json::json!(chunks)));
        Ok(m)
    }
}

// ---------------------------------------------------------- 字节分布图
struct ByteMap;
impl Node for ByteMap {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        if data.is_empty() {
            return Err(CoreError::Parse("输入为空".into()));
        }
        let w = (pnum(p, "width", 256.0).max(1.0) as usize).min(4096);
        let max_bytes = w.saturating_mul(4096).min(2_000_000);
        let (shown, note) = if data.len() > max_bytes {
            (&data[..max_bytes], format!("（仅显示前 {max_bytes} 字节）"))
        } else {
            (&data[..], String::new())
        };
        let h = shown.len().div_ceil(w).max(1);
        let mut img = RgbaImage::from_pixel(w as u32, h as u32, Rgba([0, 0, 0, 255]));
        for (idx, &b) in shown.iter().enumerate() {
            let c = byte_color(b);
            img.put_pixel((idx % w) as u32, (idx / w) as u32, Rgba([c[0], c[1], c[2], 255]));
        }
        let mut m = image_out(&img)?;
        m.insert("text".into(), PortValue::Text(format!("{w}×{h} 像素{note}")));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "byte_histogram",
            VIZ,
            "字节直方图",
            CYAN,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("image", "直方图", PortType::Image),
                opt("bytes", "PNG", PortType::Bytes),
                opt("json", "计数", PortType::Json),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(Histogram)),
    );
    reg.register(
        desc(
            "entropy_curve",
            VIZ,
            "熵曲线",
            CYAN,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("image", "熵曲线", PortType::Image),
                opt("bytes", "PNG", PortType::Bytes),
                opt("text", "概要", PortType::Text),
                opt("json", "各段熵", PortType::Json),
            ],
            vec![ParamSpec::number("window", "窗口(字节)", 16.0, 65536.0, 16.0, 256.0)],
        ),
        Arc::new(|| Arc::new(EntropyCurve)),
    );
    reg.register(
        desc(
            "byte_map",
            VIZ,
            "字节分布图",
            CYAN,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("image", "分布图", PortType::Image),
                opt("bytes", "PNG", PortType::Bytes),
                opt("text", "尺寸", PortType::Text),
            ],
            vec![ParamSpec::number("width", "宽度(像素)", 1.0, 4096.0, 1.0, 256.0)],
        ),
        Arc::new(|| Arc::new(ByteMap)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    fn run_img(id: &str, data: &[u8], params: serde_json::Value) -> String {
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(data.to_vec().into_boxed_slice())));
        let out = GraphExecutor::run_node(
            &default_registry(),
            id,
            &i,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("image") {
            Some(PortValue::Image(u)) => u.clone(),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn all_three_render_png() {
        let data: Vec<u8> = (0..2000u32).map(|x| (x * 7 + 3) as u8).collect();
        for id in ["byte_histogram", "entropy_curve", "byte_map"] {
            let url = run_img(id, &data, serde_json::json!({}));
            assert!(url.starts_with("data:image/png;base64,"), "{id}: {url:.40}");
        }
    }

    #[test]
    fn empty_input_errors() {
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(Vec::new().into_boxed_slice())));
        assert!(GraphExecutor::run_node(
            &default_registry(),
            "byte_histogram",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .is_err());
    }
}
