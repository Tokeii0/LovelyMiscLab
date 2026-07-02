//! stegpy（izcoser/stegpy）LSB 隐写提取/嵌入。
//!
//! 图像转 RGB 后扁平化（行主序、R/G/B 交错）。每个消息字节按 **低位在前** 拆成
//! `divisor = 8/bits` 组、每组 `bits` 位，依次放进 `divisor` 个宿主字节的低 `bits` 位；
//! `bits`（1/2/4）记录在第 0 个宿主字节的第 4、5 位（`(b0>>4)&3 → 2^n`）。
//! 消息体：`"stegv3"(6) + 数据长度(4 大端) + 文件名长度(1) + 文件名 + 数据`。
//! （不含密码/加密分支；JPEG DCT 模式亦不在此。）
use image::DynamicImage;

use super::image_util::{image_out, input_bytes};
use super::prelude::*;

const MAGIC: &[u8; 6] = b"stegv3";

/// 加载为 RGB 扁平字节（行主序、交错）。
fn rgb_flat(inputs: &PortMap) -> Result<(Vec<u8>, u32, u32), CoreError> {
    let bytes = input_bytes(inputs, "data")?;
    let img = image::load_from_memory(&bytes)
        .map_err(|e| CoreError::Parse(format!("图片解码失败: {e}")))?
        .to_rgb8();
    let (w, h) = img.dimensions();
    Ok((img.into_raw(), w, h))
}

/// 按 stegpy 规则从宿主字节还原消息字节序列。
fn decode(host: &[u8]) -> Vec<u8> {
    if host.is_empty() {
        return Vec::new();
    }
    let bits = 1usize << ((host[0] & 0x30) >> 4); // 2^n，n=bit4,5
    let divisor = 8 / bits.max(1);
    let mask = ((1u16 << bits) - 1) as u8;
    let count = host.len() / divisor;
    let mut msg = vec![0u8; count];
    for (k, m) in msg.iter_mut().enumerate() {
        let mut v = 0u16;
        for i in 0..divisor {
            v |= ((host[k * divisor + i] & mask) as u16) << (bits * i);
        }
        *m = v as u8;
    }
    msg
}

struct Extract;
impl Node for Extract {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let (host, _, _) = rgb_flat(i)?;
        let msg = decode(&host);
        if msg.len() < 11 || &msg[0..6] != MAGIC {
            return Err(CoreError::Parse(
                "未发现 stegpy 魔数 \"stegv3\"（可能不是 stegpy 隐写图）。".into(),
            ));
        }
        let msg_len = u32::from_be_bytes([msg[6], msg[7], msg[8], msg[9]]) as usize;
        let name_len = msg[10] as usize;
        let start = 11 + name_len;
        let end = start + msg_len;
        if end > msg.len() {
            return Err(CoreError::Parse(
                "stegpy 长度字段越界，数据可能损坏。".into(),
            ));
        }
        let filename = String::from_utf8_lossy(&msg[11..start]).into_owned();
        let data = msg[start..end].to_vec();

        let mut m = PortMap::new();
        m.insert(
            "text".into(),
            PortValue::Text(String::from_utf8_lossy(&data).into_owned()),
        );
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(data.into_boxed_slice())),
        );
        m.insert("filename".into(), PortValue::Text(filename));
        Ok(m)
    }
}

struct Embed;
impl Node for Embed {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let (mut host, w, h) = rgb_flat(i)?;
        let payload = in_bytes(i, "file")?;
        let filename = pstr(p, "filename", "");
        let bits = match pstr(p, "bits", "2") {
            "1" => 1usize,
            "4" => 4,
            _ => 2,
        };
        let divisor = 8 / bits;

        // 组装消息：MAGIC + 数据长度(4 大端) + 文件名长度(1) + 文件名 + 数据。
        let mut msg = MAGIC.to_vec();
        msg.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        let name = filename.as_bytes();
        msg.push(name.len() as u8);
        msg.extend_from_slice(name);
        msg.extend_from_slice(&payload);

        let need = msg.len() * divisor;
        if need > host.len() {
            return Err(CoreError::Other(format!(
                "图片容量不足：需 {need} 字节宿主，仅 {}。",
                host.len()
            )));
        }
        let mask = ((1u16 << bits) - 1) as u8;
        for (k, &byte) in msg.iter().enumerate() {
            for i in 0..divisor {
                let group = (byte >> (bits * i)) & mask;
                let idx = k * divisor + i;
                host[idx] = (host[idx] & !mask) | group;
            }
        }
        // 在第 0 字节第 4、5 位写入 bits 指示（log2(bits)）。
        let indicator = (bits.trailing_zeros() as u8) << 4;
        host[0] = (host[0] & 0xCF) | indicator;

        let rgb = image::RgbImage::from_raw(w, h, host)
            .ok_or_else(|| CoreError::Other("重建图片失败".into()))?;
        image_out(&DynamicImage::ImageRgb8(rgb).to_rgba8())
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "stegpy_extract",
            STEG,
            "stegpy 提取",
            PURPLE,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
                opt("filename", "文件名", PortType::Text),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(Extract)),
    );
    reg.register(
        desc(
            "stegpy_embed",
            STEG,
            "stegpy 嵌入",
            PURPLE,
            vec![
                req("data", "载体图片", PortType::Any),
                req("file", "载荷", PortType::Any),
            ],
            vec![
                req("image", "图片", PortType::Image),
                opt("bytes", "PNG字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::text("filename", "文件名(空=文本模式)", "", false),
                ParamSpec::select("bits", "每字节位数", &["1", "2", "4"], "2"),
            ],
        ),
        Arc::new(|| Arc::new(Embed)),
    );
}
