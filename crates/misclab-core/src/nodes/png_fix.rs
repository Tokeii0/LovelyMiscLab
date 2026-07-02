//! Repair a PNG whose IHDR width/height were tampered with — a classic CTF trick
//! (shrink the height so a viewer crops the bottom, hiding a flag). Two ways:
//!   • CRC 爆破: the IHDR CRC is usually left intact, so brute-force the real
//!     width/height until the stored CRC matches — an exact, lossless recovery.
//!   • 手动: force given dimensions and recompute the CRC so the file renders
//!     (use this to reveal hidden rows when the CRC was also rewritten).
//! Output is the patched **raw** bytes (no re-encode), so every other chunk and
//! all pixel data are preserved untouched.
use super::image_util::{data_url, input_bytes};
use super::prelude::*;

const SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

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

/// Find (w, h) matching `target` CRC: single-dimension fast paths first, then 2-D.
fn brute(w0: u32, h0: u32, max: u32, target: u32, tail: &[u8; 5]) -> Option<(u32, u32)> {
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

        let (new_w, new_h, note, fix_crc) = if pstr(p, "mode", "CRC 爆破") == "手动" {
            let w = pnum(p, "width", 0.0) as u32;
            let h = pnum(p, "height", 0.0) as u32;
            let w = if w == 0 { stored_w } else { w };
            let h = if h == 0 { stored_h } else { h };
            (w, h, format!("手动设置为 {w}×{h}（已重算 IHDR CRC）。"), true)
        } else if ihdr_crc(stored_w, stored_h, &tail) == stored_crc {
            return Ok(out(
                &data,
                &format!("IHDR CRC 正确（{stored_w}×{stored_h}），无需修复。"),
            ));
        } else {
            let max = (pnum(p, "max", 4096.0) as u32).max(1);
            match brute(stored_w, stored_h, max, stored_crc, &tail) {
                Some((w, h)) => (
                    w,
                    h,
                    format!("CRC 匹配：真实尺寸 {w}×{h}（原记录为 {stored_w}×{stored_h}）。"),
                    false,
                ),
                None => {
                    return Ok(out(
                        &data,
                        &format!(
                            "未在 1..={max} 内找到匹配 IHDR CRC 的尺寸（当前 {stored_w}×{stored_h}）。\
                             CRC 可能也被改写——请切到「手动」模式指定更大的高度以显示隐藏内容。"
                        ),
                    ));
                }
            }
        };

        data[16..20].copy_from_slice(&new_w.to_be_bytes());
        data[20..24].copy_from_slice(&new_h.to_be_bytes());
        if fix_crc {
            data[29..33].copy_from_slice(&ihdr_crc(new_w, new_h, &tail).to_be_bytes());
        }
        Ok(out(&data, &note))
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
                ParamSpec::select("mode", "模式", &["CRC 爆破", "手动"], "CRC 爆破"),
                ParamSpec::number("max", "爆破上限(像素)", 1.0, 65535.0, 1.0, 4096.0),
                ParamSpec::number("width", "宽(手动,0=不改)", 0.0, 1_000_000.0, 1.0, 0.0),
                ParamSpec::number("height", "高(手动,0=不改)", 0.0, 1_000_000.0, 1.0, 0.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
