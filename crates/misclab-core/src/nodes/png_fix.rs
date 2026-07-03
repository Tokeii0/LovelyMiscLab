//! Repair a PNG whose IHDR width/height were tampered with — a classic CTF trick
//! (shrink the height so a viewer crops the bottom, hiding a flag). Three ways:
//!   • 自动（默认）: decompress the IDAT stream and derive the true dimensions from
//!     the raw scanline length (`raw_len = height × stride`). This recovers the real
//!     height even when the attacker recomputed the IHDR CRC to match the fake size
//!     (the "暴力爆破" case, where CRC brute-force is blind). When the original CRC
//!     survived, a divisible candidate that also matches it gives an exact answer.
//!   • CRC 爆破: brute-force (w,h) until the stored IHDR CRC matches — exact and
//!     lossless when the CRC was left intact ("正常爆破").
//!   • 手动: force given dimensions and recompute the CRC so the file renders.
//! Output is the patched **raw** bytes (no re-encode); every other chunk and all
//! pixel data are preserved untouched.
use std::io::Read;

use flate2::read::ZlibDecoder;

use super::image_util::{data_url, input_bytes};
use super::prelude::*;

const SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
/// Practical CTF ceiling for a dimension. PNG allows up to 2^31-1, but scanning
/// that far is pointless — real challenge images are far smaller.
const MAX_DIM: u32 = 65535;

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// CRC-32 of an IHDR with the given dimensions (`b"IHDR"` + 13 data bytes).
fn ihdr_crc(w: u32, h: u32, tail: &[u8; 5]) -> u32 {
    let mut buf = [0u8; 17];
    buf[0..4].copy_from_slice(b"IHDR");
    buf[4..8].copy_from_slice(&w.to_be_bytes());
    buf[8..12].copy_from_slice(&h.to_be_bytes());
    buf[12..17].copy_from_slice(tail);
    crc32fast::hash(&buf)
}

/// Samples per pixel for a PNG colour type (bit depth is applied separately).
fn channels(color_type: u8) -> Option<u64> {
    match color_type {
        0 => Some(1), // grayscale
        2 => Some(3), // truecolour
        3 => Some(1), // indexed
        4 => Some(2), // grayscale + alpha
        6 => Some(4), // truecolour + alpha
        _ => None,
    }
}

/// Bytes in one non-interlaced scanline: 1 filter byte + ceil(bits / 8).
fn stride(w: u32, bit_depth: u8, ch: u64) -> u64 {
    let bits = w as u64 * bit_depth as u64 * ch;
    1 + bits.div_ceil(8)
}

/// Concatenate every IDAT chunk's payload, walking the chunk list.
fn collect_idat(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut o = 8usize;
    while o + 12 <= data.len() {
        let len = be32(&data[o..o + 4]) as usize;
        let typ = &data[o + 4..o + 8];
        let end = o + 8 + len;
        if end + 4 > data.len() {
            break;
        }
        if typ == b"IDAT" {
            out.extend_from_slice(&data[o + 8..end]);
        }
        if typ == b"IEND" {
            break;
        }
        o = end + 4; // skip the 4-byte chunk CRC
    }
    out
}

/// Total number of raw (decompressed) scanline bytes in the IDAT stream, or None
/// if it cannot be inflated at all. A truncated stream still yields its partial
/// length, which is enough to infer the height.
fn inflated_len(data: &[u8]) -> Option<u64> {
    let idat = collect_idat(data);
    if idat.is_empty() {
        return None;
    }
    let mut dec = ZlibDecoder::new(&idat[..]);
    let mut raw = Vec::new();
    // Ignore trailing/padding errors — read_to_end still fills `raw` with the
    // bytes it managed to decode before erroring.
    let _ = dec.read_to_end(&mut raw);
    if raw.is_empty() {
        None
    } else {
        Some(raw.len() as u64)
    }
}

/// Recover the true (width, height) from the pixel data itself. Returns the dims
/// plus a human note describing how they were found. `None` when the image is
/// interlaced (Adam7 packs scanlines differently) or the IDAT can't be inflated.
fn recover_from_data(data: &[u8]) -> Option<(u32, u32, String)> {
    let w0 = be32(&data[16..20]);
    let h0 = be32(&data[20..24]);
    let bit_depth = data[24];
    let color_type = data[25];
    let interlace = data[28];
    let tail = [data[24], data[25], data[26], data[27], data[28]];
    let stored_crc = be32(&data[29..33]);

    if interlace != 0 {
        return None; // Adam7 — leave it to the CRC brute path.
    }
    let ch = channels(color_type)?;
    if bit_depth == 0 {
        return None;
    }
    let raw = inflated_len(data)?;

    // height for a given width, only when the raw length divides evenly.
    let height_for = |w: u32| -> Option<u32> {
        let s = stride(w, bit_depth, ch);
        if s == 0 || raw % s != 0 {
            return None;
        }
        let h = raw / s;
        (1..=MAX_DIM as u64).contains(&h).then_some(h as u32)
    };

    // 1) Exact: a data-consistent (w,h) whose IHDR CRC also matches the stored one
    //    (the original CRC survived — "正常爆破"). Unambiguous.
    for w in 1..=MAX_DIM {
        if let Some(h) = height_for(w) {
            if ihdr_crc(w, h, &tail) == stored_crc {
                return Some((w, h, format!("数据流 + CRC 双重确认：真实尺寸 {w}×{h}。")));
            }
        }
    }
    // 2) CRC was rewritten ("暴力爆破"): the width is almost always untouched, so
    //    trust the stored width and read the height straight from the data.
    if let Some(h) = height_for(w0) {
        return Some((
            w0,
            h,
            format!("按 IDAT 数据流推断：宽度 {w0} 不变，真实高度 {h}（原记录高 {h0}）。"),
        ));
    }
    // 3) Fallback: the stored height is intact and only the width was scrambled.
    for w in 1..=MAX_DIM {
        if let Some(h) = height_for(w) {
            if h == h0 {
                return Some((
                    w,
                    h,
                    format!("按 IDAT 数据流推断：高度 {h0} 不变，真实宽度 {w}（原记录宽 {w0}）。"),
                ));
            }
        }
    }
    None
}

/// Find (w, h) matching `target` CRC: single-dimension fast paths first, then 2-D.
fn crc_brute(w0: u32, h0: u32, max: u32, target: u32, tail: &[u8; 5]) -> Option<(u32, u32)> {
    for h in 1..=max {
        if ihdr_crc(w0, h, tail) == target {
            return Some((w0, h));
        }
    }
    for w in 1..=max {
        if ihdr_crc(w, h0, tail) == target {
            return Some((w, h0));
        }
    }
    for w in 1..=max {
        for h in 1..=max {
            if ihdr_crc(w, h, tail) == target {
                return Some((w, h));
            }
        }
    }
    None
}

fn out(bytes: &[u8], report: &str) -> PortMap {
    let mut m = PortMap::new();
    m.insert("image".into(), PortValue::Image(data_url(bytes, "image/png")));
    m.insert(
        "bytes".into(),
        PortValue::Bytes(Arc::from(bytes.to_vec().into_boxed_slice())),
    );
    m.insert("report".into(), PortValue::Text(report.to_string()));
    m
}

/// Write dims into IHDR and refresh the IHDR CRC in place.
fn patch(data: &mut [u8], w: u32, h: u32, tail: &[u8; 5]) {
    data[16..20].copy_from_slice(&w.to_be_bytes());
    data[20..24].copy_from_slice(&h.to_be_bytes());
    data[29..33].copy_from_slice(&ihdr_crc(w, h, tail).to_be_bytes());
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut data = input_bytes(inputs, "data")?;
        // Signature(8) + IHDR: len[8..12]=13, type[12..16]="IHDR", data[16..29], crc[29..33].
        if data.len() < 33 || data[..8] != SIG {
            return Err(CoreError::Parse("不是有效的 PNG（签名缺失）。".into()));
        }
        if &data[12..16] != b"IHDR" {
            return Err(CoreError::Parse("PNG 第一个块不是 IHDR。".into()));
        }
        let stored_w = be32(&data[16..20]);
        let stored_h = be32(&data[20..24]);
        let stored_crc = be32(&data[29..33]);
        let tail = [data[24], data[25], data[26], data[27], data[28]];

        let mode = pstr(p, "mode", "自动");

        // 手动: force the given dimensions (0 = keep) and recompute the CRC.
        if mode == "手动" {
            let w = pnum(p, "width", 0.0) as u32;
            let h = pnum(p, "height", 0.0) as u32;
            let w = if w == 0 { stored_w } else { w };
            let h = if h == 0 { stored_h } else { h };
            patch(&mut data, w, h, &tail);
            return Ok(out(&data, &format!("手动设置为 {w}×{h}（已重算 IHDR CRC）。")));
        }

        // CRC 爆破: pure CRC match (exact only when the CRC was left intact).
        if mode == "CRC 爆破" {
            if ihdr_crc(stored_w, stored_h, &tail) == stored_crc {
                return Ok(out(
                    &data,
                    &format!("IHDR CRC 正确（{stored_w}×{stored_h}），无需修复。"),
                ));
            }
            let max = (pnum(p, "max", 8192.0) as u32).clamp(1, MAX_DIM);
            return match crc_brute(stored_w, stored_h, max, stored_crc, &tail) {
                Some((w, h)) => {
                    patch(&mut data, w, h, &tail);
                    Ok(out(
                        &data,
                        &format!("CRC 匹配：真实尺寸 {w}×{h}（原记录 {stored_w}×{stored_h}）。"),
                    ))
                }
                None => Ok(out(
                    &data,
                    &format!(
                        "未在 1..={max} 内找到匹配 IHDR CRC 的尺寸（当前 {stored_w}×{stored_h}）。\
                         CRC 可能也被改写——请改用「自动」模式或「手动」指定尺寸。"
                    ),
                )),
            };
        }

        // 自动（默认）: recover from the pixel data, cross-checked with the CRC.
        if let Some((w, h, note)) = recover_from_data(&data) {
            if w == stored_w && h == stored_h {
                return Ok(out(
                    &data,
                    &format!("尺寸 {w}×{h} 与像素数据一致，无需修复。"),
                ));
            }
            patch(&mut data, w, h, &tail);
            return Ok(out(&data, &note));
        }

        // Fallback for interlaced / undecodable IDAT: try the CRC brute.
        if ihdr_crc(stored_w, stored_h, &tail) == stored_crc {
            return Ok(out(
                &data,
                &format!("IHDR CRC 正确（{stored_w}×{stored_h}），且无法从数据流推断，视为无需修复。"),
            ));
        }
        match crc_brute(stored_w, stored_h, 8192, stored_crc, &tail) {
            Some((w, h)) => {
                patch(&mut data, w, h, &tail);
                Ok(out(
                    &data,
                    &format!(
                        "无法解压 IDAT，改用 CRC 爆破：真实尺寸 {w}×{h}（原 {stored_w}×{stored_h}）。"
                    ),
                ))
            }
            None => Ok(out(
                &data,
                &format!(
                    "无法从数据流或 CRC 恢复尺寸（当前 {stored_w}×{stored_h}，可能为隔行扫描或数据损坏）。\
                     请用「手动」模式指定尺寸以显示隐藏内容。"
                ),
            )),
        }
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "png_fix",
            IMG,
            "PNG 宽高修复",
            AMBER,
            vec![req("data", "PNG", PortType::Any)],
            vec![
                req("image", "修复后", PortType::Image),
                opt("bytes", "字节", PortType::Bytes),
                opt("report", "分析", PortType::Text),
            ],
            vec![
                ParamSpec::select("mode", "模式", &["自动", "CRC 爆破", "手动"], "自动"),
                ParamSpec::number("max", "CRC爆破上限(像素)", 1.0, 65535.0, 1.0, 8192.0),
                ParamSpec::number("width", "宽(手动,0=不改)", 0.0, 1_000_000.0, 1.0, 0.0),
                ParamSpec::number("height", "高(手动,0=不改)", 0.0, 1_000_000.0, 1.0, 0.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn chunk(typ: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        v.extend_from_slice(typ);
        v.extend_from_slice(data);
        let mut h = crc32fast::Hasher::new();
        h.update(typ);
        h.update(data);
        v.extend_from_slice(&h.finalize().to_be_bytes());
        v
    }

    /// A minimal valid 8-bit grayscale PNG (filter 0, zeroed pixels).
    fn make_png(w: u32, h: u32) -> Vec<u8> {
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
        let raw = vec![0u8; (1 + w as usize) * h as usize];
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(&raw).unwrap();
        let idat = e.finish().unwrap();
        let mut png = SIG.to_vec();
        png.extend(chunk(b"IHDR", &ihdr));
        png.extend(chunk(b"IDAT", &idat));
        png.extend(chunk(b"IEND", &[]));
        png
    }

    fn dims(data: &[u8]) -> (u32, u32) {
        (be32(&data[16..20]), be32(&data[20..24]))
    }

    #[test]
    fn recovers_height_when_crc_intact() {
        // "正常爆破": shrink height, leave the original CRC untouched.
        let mut png = make_png(40, 30);
        png[20..24].copy_from_slice(&5u32.to_be_bytes());
        let (w, h, _) = recover_from_data(&png).expect("should recover");
        assert_eq!((w, h), (40, 30));
    }

    #[test]
    fn recovers_height_when_crc_rewritten() {
        // "暴力爆破": shrink height AND recompute the CRC to match the fake size,
        // so a CRC brute-force can't detect the tamper.
        let mut png = make_png(40, 30);
        let tail = [png[24], png[25], png[26], png[27], png[28]];
        png[20..24].copy_from_slice(&5u32.to_be_bytes());
        png[29..33].copy_from_slice(&ihdr_crc(40, 5, &tail).to_be_bytes());
        let (w, h, _) = recover_from_data(&png).expect("should recover");
        assert_eq!((w, h), (40, 30));
    }

    #[test]
    fn recovers_scrambled_width() {
        // Only the width was changed; height is intact.
        let mut png = make_png(64, 12);
        png[16..20].copy_from_slice(&999u32.to_be_bytes());
        let (w, h, _) = recover_from_data(&png).expect("should recover");
        assert_eq!((w, h), (64, 12));
    }

    #[test]
    fn dimensions_beyond_4096_are_found() {
        // The default CRC-brute ceiling used to miss these; data recovery doesn't.
        let mut png = make_png(4097, 4097);
        png[20..24].copy_from_slice(&10u32.to_be_bytes());
        let (w, h, _) = recover_from_data(&png).expect("should recover");
        assert_eq!((w, h), (4097, 4097));
    }

    #[test]
    fn patch_writes_dims_and_valid_crc() {
        // "暴力爆破" input, then the patch a fix would apply.
        let mut png = make_png(40, 30);
        let tail = [png[24], png[25], png[26], png[27], png[28]];
        png[20..24].copy_from_slice(&5u32.to_be_bytes());
        png[29..33].copy_from_slice(&ihdr_crc(40, 5, &tail).to_be_bytes());

        let (w, h, _) = recover_from_data(&png).expect("should recover");
        patch(&mut png, w, h, &tail);
        assert_eq!(dims(&png), (40, 30));
        assert_eq!(be32(&png[29..33]), ihdr_crc(40, 30, &tail), "CRC must be valid");
    }
}
