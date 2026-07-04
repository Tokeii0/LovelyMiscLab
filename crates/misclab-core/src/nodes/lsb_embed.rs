//! LSB 嵌入（出题）：把载荷（文本/字节）写进封面图指定位平面的最低有效位，生成隐写图（PNG）。
//! 与「LSB 提取」[`super::lsb_stego`] 参数一一对应 —— 用相同的 通道顺序 / 位平面 / 位序 即可
//! 把载荷提取回来（载荷位于提取结果的开头）。用于出 misc 题：做一张 LSB 隐写图。
use std::io::Cursor;

use base64::Engine as _;

use super::prelude::*;

struct N;
impl Node for N {
    fn run(&self, inputs: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let cover = in_bytes(inputs, "cover")?;
        let mut img = image::load_from_memory(&cover)
            .map_err(|e| CoreError::Other(format!("封面图解码失败: {e}")))?
            .to_rgba8();
        let payload = in_bytes(inputs, "payload")?;

        let bit = pnum(p, "bit", 0.0).clamp(0.0, 7.0) as u8;
        let chans: Vec<usize> = pstr(p, "channels", "RGB")
            .chars()
            .filter_map(|c| match c.to_ascii_uppercase() {
                'R' => Some(0),
                'G' => Some(1),
                'B' => Some(2),
                'A' => Some(3),
                _ => None,
            })
            .collect();
        if chans.is_empty() {
            return Err(CoreError::Parse("通道至少选一个 (R/G/B/A)".into()));
        }
        let msb_first = pbool(p, "msbFirst", true);

        let total_bits = payload.len() * 8;
        let capacity = img.pixels().len() * chans.len();
        if total_bits > capacity {
            return Err(CoreError::Other(format!(
                "载荷过大：需要 {total_bits} 位，封面图仅能容纳 {capacity} 位（{}×{} 像素 × {} 通道）。",
                img.width(),
                img.height(),
                chans.len()
            )));
        }

        // Write payload bits into the chosen bit-plane of the chosen channels, in
        // row-major pixel order — the exact reading order of `lsb_extract`.
        let mut bit_idx = 0usize;
        'outer: for px in img.pixels_mut() {
            for &ch in &chans {
                if bit_idx >= total_bits {
                    break 'outer;
                }
                let byte = payload[bit_idx / 8];
                let shift = if msb_first { 7 - (bit_idx % 8) } else { bit_idx % 8 };
                let b = (byte >> shift) & 1;
                px.0[ch] = (px.0[ch] & !(1 << bit)) | (b << bit);
                bit_idx += 1;
            }
        }

        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|e| CoreError::Other(format!("PNG 编码失败: {e}")))?;
        let url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        );

        let mut out = PortMap::new();
        out.insert("image".to_string(), PortValue::Image(url));
        out.insert("bytes".to_string(), PortValue::Bytes(Arc::from(png.into_boxed_slice())));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "lsb_embed",
            STEG,
            "LSB 嵌入(出题)",
            PURPLE,
            vec![
                req("cover", "封面图", PortType::Any),
                req("payload", "载荷", PortType::Any),
            ],
            vec![
                opt("image", "隐写图", PortType::Image),
                req("bytes", "PNG 字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::text("channels", "通道顺序 (R/G/B/A)", "RGB", false),
                ParamSpec::number("bit", "位平面 (0=最低位)", 0.0, 7.0, 1.0, 0.0),
                ParamSpec::toggle("msbFirst", "高位在前打包", true),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;
    use std::io::Cursor;

    fn cover_png() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(16, 16, image::Rgba([100, 150, 200, 255]));
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png
    }

    #[test]
    fn embed_then_extract_recovers_payload() {
        let reg = default_registry();
        let params = serde_json::json!({ "channels": "RGB", "bit": 0, "msbFirst": true });

        let mut ins = PortMap::new();
        ins.insert("cover".into(), PortValue::Bytes(Arc::from(cover_png().into_boxed_slice())));
        ins.insert("payload".into(), PortValue::Text("HI".into()));
        let out = GraphExecutor::run_node(&reg, "lsb_embed", &ins, &params, &NullSink, &CancellationToken::new()).unwrap();
        let stego = match out.get("bytes") {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("{o:?}"),
        };

        // Extracting with the same params yields the payload at the start.
        let mut ins2 = PortMap::new();
        ins2.insert("data".into(), PortValue::Bytes(Arc::from(stego.into_boxed_slice())));
        let out2 = GraphExecutor::run_node(&reg, "lsb_extract", &ins2, &params, &NullSink, &CancellationToken::new()).unwrap();
        let bytes = match out2.get("bytes") {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("{o:?}"),
        };
        assert_eq!(&bytes[..2], b"HI");
    }
}
