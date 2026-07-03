//! LSB 全组合扫描（zsteg 式）：对 PNG/BMP 在一组**有界**的组合上尝试 LSB 提取并打分——
//! 位平面(0..=maxBit) × 通道子集{r,g,b,rgb,bgr} × 位序(MSB/LSB 先) × 遍历(行/可选列)，
//! 每个候选流转字节后按可读性 + flag 正则打分，返回排名靠前的若干个。默认 10 个组合。
use image::RgbaImage;

use super::image_util::load_image;
use super::prelude::*;

/// 通道组合（名字, 通道下标序列）。R=0 G=1 B=2 A=3。
const CHANNEL_COMBOS: &[(&str, &[usize])] = &[
    ("r", &[0]),
    ("g", &[1]),
    ("b", &[2]),
    ("rgb", &[0, 1, 2]),
    ("bgr", &[2, 1, 0]),
];

/// 按给定组合提取 LSB 比特并打包成字节（上限 `max_bytes`）。
fn extract(
    img: &RgbaImage,
    bit: u8,
    chans: &[usize],
    msb_first: bool,
    col_major: bool,
    max_bytes: usize,
) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let cap_bits = max_bytes * 8;
    let mut bits: Vec<u8> = Vec::with_capacity(cap_bits);
    let coords: Box<dyn Iterator<Item = (u32, u32)>> = if col_major {
        Box::new((0..w).flat_map(move |x| (0..h).map(move |y| (x, y))))
    } else {
        Box::new((0..h).flat_map(move |y| (0..w).map(move |x| (x, y))))
    };
    for (x, y) in coords {
        let px = img.get_pixel(x, y).0;
        for &c in chans {
            bits.push((px[c] >> bit) & 1);
        }
        if bits.len() >= cap_bits {
            break;
        }
    }
    bits.chunks_exact(8)
        .map(|c| {
            let mut byte = 0u8;
            for (k, &b) in c.iter().enumerate() {
                byte |= b << (if msb_first { 7 - k } else { k });
            }
            byte
        })
        .collect()
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let img = load_image(i, "data")?;
        let max_bit = (pnum(p, "maxBit", 0.0).max(0.0) as u8).min(7);
        let col = pbool(p, "columnMajor", false);
        let re = {
            let pat = pstr(p, "flagRegex", "flag\\{");
            if pat.is_empty() {
                None
            } else {
                regex::Regex::new(pat).ok()
            }
        };
        let max_bytes = 2048usize;
        let traversals: &[bool] = if col { &[false, true] } else { &[false] };

        let mut cands: Vec<ScoredString> = Vec::new();
        for bit in 0..=max_bit {
            for (cname, chans) in CHANNEL_COMBOS {
                for &msb in &[true, false] {
                    for &cm in traversals {
                        let bytes = extract(&img, bit, chans, msb, cm, max_bytes);
                        let text = String::from_utf8_lossy(&bytes).into_owned();
                        let mut score = english_score(&text);
                        if let Some(re) = &re {
                            if re.is_match(&text) {
                                score += 5.0;
                            }
                        }
                        let note = format!(
                            "bit{bit} {cname} {}{}",
                            if msb { "msb" } else { "lsb" },
                            if cm { " 列优先" } else { "" }
                        );
                        cands.push(ScoredString {
                            text,
                            score,
                            note: Some(note),
                        });
                    }
                }
            }
        }
        cands.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        cands.truncate(8);

        let best = cands.first().map(|c| c.text.clone()).unwrap_or_default();
        let mut out = PortMap::new();
        out.insert("best".into(), PortValue::Text(best));
        out.insert("candidates".into(), PortValue::Candidates(cands));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "zsteg_scan",
            STEG,
            "LSB 全组合扫描",
            PURPLE,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("best", "最佳候选", PortType::Text),
                opt("candidates", "候选(带打分)", PortType::Candidates),
            ],
            vec![
                ParamSpec::number("maxBit", "最大 bit 位(0..7)", 0.0, 7.0, 1.0, 0.0),
                ParamSpec::toggle("columnMajor", "含列优先遍历", false),
                ParamSpec::text("flagRegex", "flag 正则", "flag\\{", false),
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

    /// 把 `msg` 的比特（MSB 先）写进 R 通道 LSB（行优先），编码为 PNG 字节。
    fn embed_r_lsb(msg: &[u8]) -> Vec<u8> {
        let bits: Vec<u8> = msg
            .iter()
            .flat_map(|&b| (0..8).map(move |k| (b >> (7 - k)) & 1))
            .collect();
        let mut img = RgbaImage::from_pixel(128, 1, image::Rgba([0, 0, 0, 255]));
        for (i, &bit) in bits.iter().enumerate() {
            let px = img.get_pixel_mut(i as u32, 0);
            px.0[0] = (px.0[0] & 0xFE) | bit;
        }
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png
    }

    #[test]
    fn finds_r_lsb_flag() {
        let png = embed_r_lsb(b"flag{lsb}");
        let mut inputs = PortMap::new();
        inputs.insert(
            "data".into(),
            PortValue::Bytes(Arc::from(png.into_boxed_slice())),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "zsteg_scan",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        let best = match out.get("best") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        };
        assert!(best.contains("flag{lsb}"), "best = {best:?}");
    }
}
