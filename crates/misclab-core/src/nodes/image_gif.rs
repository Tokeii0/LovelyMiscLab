//! GIF multi-frame handling: extract a single frame, or lay all frames out as a
//! sprite sheet (each animation frame is a common place to hide data).
use std::io::Cursor;

use image::{AnimationDecoder, RgbaImage};

use super::image_util::*;
use super::prelude::*;

fn frames(bytes: &[u8]) -> Result<Vec<RgbaImage>, CoreError> {
    let dec = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
        .map_err(|e| CoreError::Parse(format!("GIF 解码失败: {e}")))?;
    let fs = dec
        .into_frames()
        .collect_frames()
        .map_err(|e| CoreError::Parse(format!("GIF 帧解码失败: {e}")))?;
    if fs.is_empty() {
        return Err(CoreError::Other("GIF 无帧".into()));
    }
    Ok(fs.into_iter().map(|f| f.into_buffer()).collect())
}

struct GifFrame;
impl Node for GifFrame {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let fs = frames(&input_bytes(i, "data")?)?;
        let idx = (pnum(p, "index", 0.0).max(0.0) as usize).min(fs.len() - 1);
        let mut m = image_out(&fs[idx])?;
        m.insert("count".to_string(), PortValue::Number(fs.len() as f64));
        Ok(m)
    }
}

struct GifSprite;
impl Node for GifSprite {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let fs = frames(&input_bytes(i, "data")?)?;
        let cols = (pnum(p, "columns", 8.0).max(1.0) as u32).min(fs.len() as u32);
        let (fw, fh) = fs[0].dimensions();
        let rows = (fs.len() as u32).div_ceil(cols);
        let mut out = RgbaImage::new(cols * fw, rows * fh);
        for (idx, f) in fs.iter().enumerate() {
            let cx = (idx as u32 % cols) * fw;
            let cy = (idx as u32 / cols) * fh;
            image::imageops::overlay(&mut out, f, cx as i64, cy as i64);
        }
        let mut m = image_out(&out)?;
        m.insert("count".to_string(), PortValue::Number(fs.len() as f64));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let dout = || {
        vec![
            req("image", "图片", PortType::Image),
            opt("bytes", "字节", PortType::Bytes),
            opt("count", "帧数", PortType::Number),
        ]
    };
    reg.register(
        desc(
            "gif_frame",
            IMG,
            "GIF 取帧",
            FUCHSIA,
            vec![req("data", "GIF", PortType::Any)],
            dout(),
            vec![ParamSpec::number("index", "帧序号", 0.0, 100000.0, 1.0, 0.0)],
        ),
        Arc::new(|| Arc::new(GifFrame)),
    );
    reg.register(
        desc(
            "gif_sprite",
            IMG,
            "GIF 拼帧",
            FUCHSIA,
            vec![req("data", "GIF", PortType::Any)],
            dout(),
            vec![ParamSpec::number("columns", "每行帧数", 1.0, 64.0, 1.0, 8.0)],
        ),
        Arc::new(|| Arc::new(GifSprite)),
    );
}
