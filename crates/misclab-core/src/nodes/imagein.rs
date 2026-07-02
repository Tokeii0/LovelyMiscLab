//! imageIN（图影 / nullice·hinaLayer）LSB 文件隐写：把一个文件连同原始文件名嵌入
//! 图像低位平面，或从这样的图像中还原出文件。
//!
//! 参考 C++ 工具 `nullice/hinaLayer` 的位写入/文件容器。要点：
//! - 通道按 OpenCV 的 **BGR** 顺序遍历（0=B,1=G,2=R）；本工程用 `image` 的 RGBA，
//!   通过 [`CV2RGBA`] 映射（0→2,1→1,2→0）。
//! - 每个通道从最低位 bit0 向上取 `depth` 个比特；字节按 **高位在前** 拆分/打包。
//! - 像素 **行主序** 遍历。文件容器：`FF FE | 文件数据 | FE FF | 文件名 | FE FF 98 0A 14 FD FE FF`。
//!
//! 现实中存在两种排布（本节点提取时自动识别，二者都用同一自描述容器）：
//! 1. **GUI 版**：从 (0,0) 起、无深度标记（实测真实样本 `个人账单.xlsx`，深度 1，GBK 名）。
//! 2. **控制台 `bit_f_write_A`**：跳过 (0,0)，深度记在 (0,0) 蓝色通道个位（样本
//!    `tests/fixtures/imagein_rr_INfile1.png` → `100.txt`）。
//!
//! 容器自带 8 字节结尾标记，故可对 `(是否跳过首像素, 深度)` 组合逐一尝试，命中即真。
use image::RgbaImage;

use super::image_util::{image_out, load_image};
use super::prelude::*;

/// 文件头（2 字节）。
const HEADER: [u8; 2] = [0xFF, 0xFE];
/// 文件名分隔（2 字节）。
const NAME_SEP: [u8; 2] = [0xFE, 0xFF];
/// 结尾标记（8 字节），来自 hinaLayer `ins_hide_file`。
const END_MARKER: [u8; 8] = [0xFE, 0xFF, 0x98, 0x0A, 0x14, 0xFD, 0xFE, 0xFF];
/// OpenCV 通道号 → `image` RGBA 下标：0=B→2, 1=G→1, 2=R→0。
const CV2RGBA: [usize; 3] = [2, 1, 0];

/// 把通道参数解析为要处理的 OpenCV 通道号列表（默认全部，BGR 顺序）。
fn channels(sel: &str) -> Vec<usize> {
    match sel {
        "B(蓝)" => vec![0],
        "G(绿)" => vec![1],
        "R(红)" => vec![2],
        _ => vec![0, 1, 2],
    }
}

/// 在 `hay` 中查找 `needle` 最后一次出现的位置（对应 C++ `rfind`）。
fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (0..=hay.len() - needle.len())
        .rev()
        .find(|&i| &hay[i..i + needle.len()] == needle)
}

/// 从图像低位平面提取比特流：行主序、可选跳过 (0,0)，每通道自 bit0 起取 `depth` 位，
/// 字节高位在前打包。`max_bytes` 用于只窥探开头（避免为每个候选都扫全图）。
fn extract_stream(
    img: &RgbaImage,
    chans: &[usize],
    skip00: bool,
    depth: u8,
    max_bytes: Option<usize>,
) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let cap = max_bytes.unwrap_or(usize::MAX);
    let mut out = Vec::new();
    let mut cur = 0u8;
    let mut nbits = 0u32;
    'outer: for y in 0..h {
        for x in 0..w {
            if skip00 && x == 0 && y == 0 {
                continue;
            }
            let px = img.get_pixel(x, y).0;
            for &c in chans {
                let val = px[CV2RGBA[c]];
                for z in 0..depth {
                    cur |= ((val >> z) & 1) << (7 - nbits);
                    nbits += 1;
                    if nbits == 8 {
                        out.push(cur);
                        cur = 0;
                        nbits = 0;
                        if out.len() >= cap {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
    out
}

/// (0,0) 蓝色通道个位记录的深度（控制台变深度格式）：`blue % 10 + 2`。
fn marker_depth(img: &RgbaImage) -> u8 {
    (img.get_pixel(0, 0).0[CV2RGBA[0]] % 10 + 2).min(8)
}

/// 解码文件名：优先按 UTF-8，失败则按 GB18030（imageIN 多在 Windows 中文环境生成，
/// 文件名常为 GBK 编码）。
fn decode_name(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::GB18030.decode(raw).0.into_owned(),
    }
}

/// 解析文件容器：`FF FE | 数据 | FE FF | 文件名 | 结尾标记`。返回 (文件名, 文件数据)。
fn parse_container(raw: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if raw.len() < 2 || raw[..2] != HEADER {
        return None;
    }
    let end = rfind(raw, &END_MARKER)?;
    let body = &raw[2..end]; // 数据 | FE FF | 文件名
    let sep = rfind(body, &NAME_SEP)?;
    Some((body[sep + 2..].to_vec(), body[..sep].to_vec()))
}

/// 候选 `(是否跳过首像素, 深度)`，按命中概率排序：GUI(含首像素,深度1) → 控制台(跳过,标记
/// 深度) → 其余深度兜底。
fn candidates(marker: u8, forced: u8) -> Vec<(bool, u8)> {
    if forced > 0 {
        let d = forced.min(8);
        return vec![(false, d), (true, d)];
    }
    let mut v = vec![(false, 1u8), (true, marker)];
    for d in 1..=8u8 {
        v.push((false, d));
        v.push((true, d));
    }
    v
}

// ---------------------------------------------------------------- 提取
struct Extract;
impl Node for Extract {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let chans = channels(pstr(p, "channels", "全部(BGR)"));
        let forced = pnum(p, "depth", 0.0).clamp(0.0, 8.0) as u8;

        let cands = candidates(marker_depth(&img), forced);
        // 逐候选尝试：先窥探 2 字节文件头，命中才扫全图并解析容器。
        let hit = cands.iter().find_map(|&(skip, d)| {
            if d == 0 || extract_stream(&img, &chans, skip, d, Some(2)) != HEADER {
                return None;
            }
            parse_container(&extract_stream(&img, &chans, skip, d, None))
                .map(|(name, data)| (skip, d, name, data))
        });

        let mut m = PortMap::new();
        match hit {
            Some((skip, depth, name, data)) => {
                let filename = decode_name(&name);
                let start = if skip { "跳过(0,0)" } else { "含(0,0)" };
                let report = format!(
                    "imageIN 文件容器：深度={depth}，起始{start}，文件名=\"{filename}\"，大小={} 字节。",
                    data.len()
                );
                m.insert(
                    "text".into(),
                    PortValue::Text(String::from_utf8_lossy(&data).into_owned()),
                );
                m.insert(
                    "bytes".into(),
                    PortValue::Bytes(Arc::from(data.into_boxed_slice())),
                );
                m.insert("filename".into(), PortValue::Text(filename));
                m.insert("report".into(), PortValue::Text(report));
            }
            None => {
                // 没找到容器：返回首选排布的原始比特流，便于手动分析（换通道/深度重试）。
                let (skip, d) = cands[0];
                let raw = extract_stream(&img, &chans, skip, d.max(1), None);
                let report =
                    "未发现 imageIN 文件容器。已返回原始低位比特流，可尝试更换通道或深度。"
                        .to_string();
                m.insert(
                    "text".into(),
                    PortValue::Text(String::from_utf8_lossy(&raw).into_owned()),
                );
                m.insert(
                    "bytes".into(),
                    PortValue::Bytes(Arc::from(raw.into_boxed_slice())),
                );
                m.insert("filename".into(), PortValue::Text(String::new()));
                m.insert("report".into(), PortValue::Text(report));
            }
        }
        Ok(m)
    }
}

// ---------------------------------------------------------------- 嵌入
/// 构造 imageIN 文件容器（与真实工具字节一致）。
fn build_container(payload: &[u8], filename: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(payload.len() + filename.len() + 12);
    v.extend_from_slice(&HEADER);
    v.extend_from_slice(payload);
    v.extend_from_slice(&NAME_SEP);
    v.extend_from_slice(filename.as_bytes());
    v.extend_from_slice(&END_MARKER);
    v
}

/// 自动深度：`ceil(size / (pxbit/8))`，clamp 到 [1,8]。
fn auto_depth(size: usize, pxbit: usize) -> u8 {
    if size == 0 || pxbit == 0 {
        return 1;
    }
    let deep = (size as f64 / (pxbit as f64 / 8.0)).ceil() as i64;
    deep.clamp(1, 8) as u8
}

struct Embed;
impl Node for Embed {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut img = load_image(i, "data")?;
        let payload = in_bytes(i, "file")?;
        let filename = pstr(p, "filename", "secret.bin");
        let chans = channels(pstr(p, "channels", "全部(BGR)"));
        let (w, h) = img.dimensions();

        // GUI 版排布：从 (0,0) 起、无深度标记（提取端自动识别深度）。
        let container = build_container(&payload, filename);
        let pxbit = w as usize * h as usize * chans.len();
        let forced = pnum(p, "depth", 0.0).clamp(0.0, 8.0) as u8;
        let depth = if forced == 0 {
            auto_depth(container.len(), pxbit)
        } else {
            forced.clamp(1, 8)
        };

        let need_bits = container.len() * 8;
        let capacity_bits = pxbit * depth as usize;
        if need_bits > capacity_bits {
            return Err(CoreError::Other(format!(
                "图片容量不足：需要 {need_bits} 位，最多 {capacity_bits} 位（深度 {depth}）。请换更大的图或提高深度。"
            )));
        }

        // 逐位写入：行主序，每通道从 bit0 起写 depth 位，源字节高位在前。
        let mut ci = 0usize;
        let mut bpos = 0u32;
        'outer: for y in 0..h {
            for x in 0..w {
                let px = img.get_pixel_mut(x, y);
                for &c in &chans {
                    let b = CV2RGBA[c];
                    let mut val = px.0[b];
                    for z in 0..depth {
                        if ci >= container.len() {
                            px.0[b] = val; // 冲刷本通道已写入的部分比特再退出
                            break 'outer;
                        }
                        if (container[ci] >> (7 - bpos)) & 1 == 1 {
                            val |= 1 << z;
                        } else {
                            val &= !(1 << z);
                        }
                        bpos += 1;
                        if bpos == 8 {
                            bpos = 0;
                            ci += 1;
                        }
                    }
                    px.0[b] = val;
                }
            }
        }

        let mut m = image_out(&img)?;
        m.insert(
            "report".into(),
            PortValue::Text(format!(
                "已嵌入 {} 字节（含容器），深度={depth}，文件名=\"{filename}\"。",
                container.len()
            )),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let chan_opts = &["全部(BGR)", "B(蓝)", "G(绿)", "R(红)"];
    reg.register(
        desc(
            "imagein_extract",
            STEG,
            "imageIN 文件提取",
            PURPLE,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "文本预览", PortType::Text),
                opt("bytes", "文件字节", PortType::Bytes),
                opt("filename", "文件名", PortType::Text),
                opt("report", "信息", PortType::Text),
            ],
            vec![
                ParamSpec::select("channels", "通道", chan_opts, "全部(BGR)"),
                ParamSpec::number("depth", "深度(0=自动识别)", 0.0, 8.0, 1.0, 0.0),
            ],
        ),
        Arc::new(|| Arc::new(Extract)),
    );
    reg.register(
        desc(
            "imagein_embed",
            STEG,
            "imageIN 文件嵌入",
            PURPLE,
            vec![
                req("data", "载体图片", PortType::Any),
                req("file", "要嵌入的文件", PortType::Any),
            ],
            vec![
                req("image", "图片", PortType::Image),
                opt("bytes", "PNG字节", PortType::Bytes),
                opt("report", "信息", PortType::Text),
            ],
            vec![
                ParamSpec::text("filename", "记录的文件名", "secret.bin", false),
                ParamSpec::select("channels", "通道", chan_opts, "全部(BGR)"),
                ParamSpec::number("depth", "深度(0=自动)", 0.0, 8.0, 1.0, 0.0),
            ],
        ),
        Arc::new(|| Arc::new(Embed)),
    );
}
