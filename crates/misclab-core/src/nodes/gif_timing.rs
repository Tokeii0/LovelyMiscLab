//! GIF 帧时序解码：flag 常藏在每帧的显示时长里。读每帧 delay（厘秒 = centisecond），
//! 按所选模式映射为输出：原始数字 / 当字节转 ASCII / 阈值二值化转 bit。
use std::io::Cursor;

use image::AnimationDecoder;

use super::image_util::input_bytes;
use super::prelude::*;

/// 每帧显示时长（厘秒）。`image` 的 GIF 解码把 GCE 的 cs 值转成 ms 分数，这里再折回 cs。
fn delays_cs(bytes: &[u8]) -> Result<Vec<u32>, CoreError> {
    let dec = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
        .map_err(|e| CoreError::Parse(format!("GIF 解码失败: {e}")))?;
    let frames = dec
        .into_frames()
        .collect_frames()
        .map_err(|e| CoreError::Parse(format!("GIF 帧解码失败: {e}")))?;
    if frames.is_empty() {
        return Err(CoreError::Other("GIF 无帧".into()));
    }
    Ok(frames
        .iter()
        .map(|f| {
            let (n, d) = f.delay().numer_denom_ms();
            if d == 0 {
                0
            } else {
                ((n as f64 / d as f64) / 10.0).round() as u32
            }
        })
        .collect())
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let cs = delays_cs(&input_bytes(i, "data")?)?;
        let delays_str = cs
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        let text = match pstr(p, "mode", "字节/ASCII") {
            "原始数字" => delays_str.clone(),
            "二进制" => {
                let thr = pnum(p, "threshold", 5.0).round() as u32;
                let bits: String = cs.iter().map(|&v| if v >= thr { '1' } else { '0' }).collect();
                let bytes: Vec<u8> = bits
                    .as_bytes()
                    .chunks(8)
                    .filter(|c| c.len() == 8)
                    .map(|c| c.iter().fold(0u8, |acc, &b| (acc << 1) | u8::from(b == b'1')))
                    .collect();
                format!("{bits}\n→ {}", String::from_utf8_lossy(&bytes))
            }
            _ => {
                // 字节/ASCII：每帧 cs 当一个字节。
                let bytes: Vec<u8> = cs.iter().map(|&v| v as u8).collect();
                String::from_utf8_lossy(&bytes).into_owned()
            }
        };

        let raw: Vec<u8> = cs.iter().map(|&v| v as u8).collect();
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(raw.into_boxed_slice())),
        );
        m.insert("delays".into(), PortValue::Text(delays_str));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "gif_timing",
            IMG,
            "GIF 帧时序解码",
            AMBER,
            vec![req("data", "GIF", PortType::Any)],
            vec![
                req("text", "结果", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
                opt("delays", "帧时长(厘秒)", PortType::Text),
            ],
            vec![
                ParamSpec::select(
                    "mode",
                    "映射",
                    &["原始数字", "字节/ASCII", "二进制"],
                    "字节/ASCII",
                ),
                ParamSpec::number("threshold", "二进制阈值(厘秒)", 0.0, 1000.0, 1.0, 5.0),
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
    use image::{codecs::gif::GifEncoder, Delay, Frame, RgbaImage};

    /// 造一个每帧时长（厘秒）由 `cs` 指定的 1×1 GIF。
    fn make_gif(cs: &[u32]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut enc = GifEncoder::new(&mut buf);
            for &c in cs {
                let img = RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
                let frame = Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(c * 10, 1));
                enc.encode_frame(frame).unwrap();
            }
        }
        buf
    }

    #[test]
    fn decodes_delays_to_ascii() {
        // "Hi" = [72, 105] 厘秒。
        let gif = make_gif(&[72, 105]);
        let mut inputs = PortMap::new();
        inputs.insert(
            "data".into(),
            PortValue::Bytes(Arc::from(gif.into_boxed_slice())),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "gif_timing",
            &inputs,
            &serde_json::json!({"mode":"字节/ASCII"}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(
            match out.get("text") {
                Some(PortValue::Text(s)) => s.clone(),
                o => panic!("{o:?}"),
            },
            "Hi"
        );
    }
}
