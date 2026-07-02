//! PixelJihad（oakes/PixelJihad）提取/嵌入。
//!
//! 消息（JSON 字符串）按 **16 位、低位在前** 藏进图像颜色字节的最低位，位置由
//! `SHA256(password)`（8 个有符号 32 位字）伪随机决定，且跳过 Alpha 字节。流开头是
//! 16 位的字符数，随后每个字符 16 位（UTF-16 码元）。无密码时明文为 `{"text": ...}`；
//! 有密码时是 SJCL 密文 JSON（`{"ct": ...}`，本节点只提取、不做 SJCL-CCM 解密）。
use image::RgbaImage;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

use super::image_util::{image_out, load_image};
use super::prelude::*;

const MAX_MESSAGE: usize = 1000;

/// SHA256(password) → 8 个有符号 32 位字（大端）。
fn pw_hash(password: &str) -> [i64; 8] {
    let d = Sha256::digest(password.as_bytes());
    let mut h = [0i64; 8];
    for (i, w) in h.iter_mut().enumerate() {
        *w = i32::from_be_bytes([d[i * 4], d[i * 4 + 1], d[i * 4 + 2], d[i * 4 + 3]]) as i64;
    }
    h
}

/// 下一个存放比特的位置（复刻 PixelJihad getNextLocation）。
fn next_location(used: &mut HashSet<usize>, hash: &[i64; 8], total: usize) -> usize {
    let pos = used.len() as i64;
    let mut loc = ((hash[(pos as usize) % 8] * (pos + 1)).abs() % total as i64) as usize;
    loop {
        if loc >= total {
            loc = 0;
        } else if used.contains(&loc) {
            loc += 1;
        } else if (loc + 1).is_multiple_of(4) {
            loc += 1; // 跳过 alpha 字节
        } else {
            used.insert(loc);
            return loc;
        }
    }
}

/// 读一个 16 位数（低位在前）。
fn read_u16(colors: &[u8], used: &mut HashSet<usize>, hash: &[i64; 8]) -> u16 {
    let mut n = 0u16;
    for pos in 0..16 {
        let loc = next_location(used, hash, colors.len());
        n |= ((colors[loc] & 1) as u16) << pos;
    }
    n
}

fn write_bit(colors: &mut [u8], used: &mut HashSet<usize>, hash: &[i64; 8], bit: u8) {
    let loc = next_location(used, hash, colors.len());
    colors[loc] = (colors[loc] & !1) | (bit & 1);
    // 该像素的 alpha 置 255（原工具因预乘 alpha 而必须这么做）。
    let alpha = loc | 3; // 本像素的 alpha 字节下标（(a+1)%4==0）
    if alpha < colors.len() {
        colors[alpha] = 255;
    }
}

struct Extract;
impl Node for Extract {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let colors = img.into_raw();
        let hash = pw_hash(pstr(p, "password", ""));
        let mut used = HashSet::new();

        let size = read_u16(&colors, &mut used, &hash) as usize;
        if size == 0 || size > MAX_MESSAGE || (size + 1) * 16 > colors.len() * 3 / 4 {
            return Err(CoreError::Parse(
                "未发现 PixelJihad 数据（大小无效或密码错误）。".into(),
            ));
        }
        let units: Vec<u16> = (0..size)
            .map(|_| read_u16(&colors, &mut used, &hash))
            .collect();
        let json = String::from_utf16_lossy(&units);

        // 解析 JSON：{"text":...} 直接取文本；{"ct":...} 为 SJCL 密文，原样返回。
        let mut m = PortMap::new();
        match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(v) if v.get("text").and_then(|t| t.as_str()).is_some() => {
                let text = v["text"].as_str().unwrap().to_string();
                m.insert("text".into(), PortValue::Text(text));
                m.insert("json".into(), PortValue::Text(json));
            }
            Ok(v) if v.get("ct").is_some() => {
                m.insert(
                    "text".into(),
                    PortValue::Text(format!(
                        "[SJCL 加密] 请用密码 + SJCL 解密下方密文：\n{json}"
                    )),
                );
                m.insert("json".into(), PortValue::Text(json));
            }
            _ => {
                return Err(CoreError::Parse(
                    "提取到的数据不是有效 JSON（密码错误或非 PixelJihad 图）。".into(),
                ));
            }
        }
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
        let img = load_image(i, "data")?;
        let (w, h) = img.dimensions();
        let mut colors = img.into_raw();
        let password = pstr(p, "password", "");
        if !password.is_empty() {
            return Err(CoreError::Other(
                "暂不支持带密码嵌入（需 SJCL 加密）。请用无密码模式，或用原工具加密。".into(),
            ));
        }
        let message = pstr(p, "message", "");
        // 无密码：明文为 {"text": message}（与 JS JSON.stringify 一致）。
        let json = format!(
            "{{\"text\":{}}}",
            serde_json::Value::String(message.to_string())
        );
        let units: Vec<u16> = json.encode_utf16().collect();
        if units.len() > MAX_MESSAGE {
            return Err(CoreError::Other("消息过长。".into()));
        }
        if (units.len() + 1) * 16 > colors.len() * 3 / 4 {
            return Err(CoreError::Other("消息过长，图片装不下。".into()));
        }

        let hash = pw_hash(password);
        let mut used = HashSet::new();
        // 先写 16 位长度，再写每个字符 16 位（低位在前）。
        let write_u16 = |colors: &mut [u8], used: &mut HashSet<usize>, n: u16| {
            for pos in 0..16 {
                write_bit(colors, used, &hash, ((n >> pos) & 1) as u8);
            }
        };
        write_u16(&mut colors, &mut used, units.len() as u16);
        for u in &units {
            write_u16(&mut colors, &mut used, *u);
        }
        image_out(
            &RgbaImage::from_raw(w, h, colors)
                .ok_or_else(|| CoreError::Other("重建图片失败".into()))?,
        )
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "pixeljihad_extract",
            STEG,
            "PixelJihad 提取",
            PURPLE,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "文本", PortType::Text),
                opt("json", "原始JSON", PortType::Text),
            ],
            vec![ParamSpec::text("password", "密码", "", false)],
        ),
        Arc::new(|| Arc::new(Extract)),
    );
    reg.register(
        desc(
            "pixeljihad_embed",
            STEG,
            "PixelJihad 嵌入",
            PURPLE,
            vec![req("data", "载体图片", PortType::Any)],
            vec![
                req("image", "图片", PortType::Image),
                opt("bytes", "PNG字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::text("message", "要隐写的文本", "", false),
                ParamSpec::text("password", "密码(暂仅支持空)", "", false),
            ],
        ),
        Arc::new(|| Arc::new(Embed)),
    );
}
