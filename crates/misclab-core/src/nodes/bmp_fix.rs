//! 修复被篡改宽/高的 BMP —— 和 png_fix 同类的 CTF 套路（改小高度让查看器裁掉底部藏 flag）。
//! BMP 无 CRC，但**未压缩**位图的像素区大小由 宽/高/位深 唯一决定：
//!   `rowSize = ((bpp*width + 31)/32)*4`，  像素字节 = `rowSize * |height|`。
//! 于是用「文件里实际的像素字节数」反推真实尺寸：
//!   • 自动：先信任宽度、由像素区整除行长得到真实高度；否则精确爆破宽度。
//!   • 手动：直接写入给定宽/高。
//! 输出修复后的原始字节（只改 0x12 宽、0x16 高两个字段，其余原样保留）。
use super::image_util::{data_url, input_bytes};
use super::prelude::*;

const MAX_DIM: u32 = 65535;

fn u16le(d: &[u8], o: usize) -> u32 {
    d[o] as u32 | (d[o + 1] as u32) << 8
}
fn u32le(d: &[u8], o: usize) -> u32 {
    u16le(d, o) | u16le(d, o + 2) << 16
}

/// 每行字节数（补齐到 4 字节）。
fn row_size(w: u32, bpp: u32) -> u64 {
    (bpp as u64 * w as u64).div_ceil(32) * 4
}

fn out(bytes: &[u8], report: &str) -> PortMap {
    let mut m = PortMap::new();
    m.insert("image".into(), PortValue::Image(data_url(bytes, "image/bmp")));
    m.insert(
        "bytes".into(),
        PortValue::Bytes(Arc::from(bytes.to_vec().into_boxed_slice())),
    );
    m.insert("report".into(), PortValue::Text(report.to_string()));
    m
}

/// 把宽/高写回 BMP 头（0x12 宽、0x16 高，均 i32 LE；高保留原符号）。
fn patch(d: &mut [u8], w: u32, h_signed: i32) {
    d[0x12..0x16].copy_from_slice(&(w as i32).to_le_bytes());
    d[0x16..0x1A].copy_from_slice(&h_signed.to_le_bytes());
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut d = input_bytes(i, "data")?;
        if d.len() < 0x1E || &d[0..2] != b"BM" {
            return Err(CoreError::Parse("不是有效的 BMP（缺少 'BM' 头）。".into()));
        }
        let off = u32le(&d, 0x0A) as usize; // bfOffBits：像素数据偏移
        let stored_w = u32le(&d, 0x12) as i32; // biWidth
        let stored_h = u32le(&d, 0x16) as i32; // biHeight（可负=自上而下）
        let bpp = u16le(&d, 0x1C); // biBitCount
        let compression = u32le(&d, 0x1E); // biCompression
        if bpp == 0 {
            return Err(CoreError::Parse("BMP 位深为 0，无法处理。".into()));
        }
        if off >= d.len() {
            return Err(CoreError::Parse("像素数据偏移越界，BMP 头可能损坏。".into()));
        }

        let mode = pstr(p, "mode", "自动");
        if mode == "手动" {
            let w = pnum(p, "width", 0.0) as u32;
            let h = pnum(p, "height", 0.0) as u32;
            let w = if w == 0 { stored_w.unsigned_abs() } else { w };
            let h = if h == 0 { stored_h.unsigned_abs() } else { h };
            let h_signed = if stored_h < 0 { -(h as i32) } else { h as i32 };
            patch(&mut d, w, h_signed);
            return Ok(out(&d, &format!("手动设置为 {w}×{h}。")));
        }

        // 自动：仅未压缩位图有确定的像素区大小。
        if compression != 0 {
            return Err(CoreError::Parse(
                "该 BMP 为压缩格式（biCompression≠0），无法由大小反推尺寸，请用「手动」模式。"
                    .into(),
            ));
        }
        let avail = (d.len() - off) as u64;
        let sw = stored_w.unsigned_abs();
        let sh = stored_h.unsigned_abs();
        let rs_w = row_size(sw, bpp);

        // 一致性（宽松：允许 <1 行的尾部字节）→ 无需修复。
        let fits = |w: u32, h: u32| -> bool {
            let rs = row_size(w, bpp);
            rs > 0 && h >= 1 && rs * (h as u64) <= avail && avail - rs * (h as u64) < rs
        };
        if sw >= 1 && fits(sw, sh) {
            return Ok(out(
                &d,
                &format!("尺寸 {sw}×{sh} 与像素数据一致，无需修复。"),
            ));
        }

        // 1) 信任宽度，向下取整得真实行数（高度被改，最常见）。
        if sw >= 1 && rs_w > 0 {
            let h = avail / rs_w;
            if (1..=MAX_DIM as u64).contains(&h) && h as u32 != sh {
                let h = h as u32;
                let h_signed = if stored_h < 0 { -(h as i32) } else { h as i32 };
                patch(&mut d, sw, h_signed);
                let note = if avail.is_multiple_of(rs_w) {
                    ""
                } else {
                    "（像素区末尾有多余字节，按整数行取高）"
                };
                return Ok(out(
                    &d,
                    &format!("按像素数据推断：宽 {sw} 不变，真实高 {h}（原记录 {sh}）{note}。"),
                ));
            }
        }
        // 2) 信任高度，精确爆破宽度（宽度被改）。
        if sh >= 1 {
            for w in 1..=MAX_DIM {
                let rs = row_size(w, bpp);
                if rs > 0 && avail.is_multiple_of(rs) && (avail / rs) as u32 == sh && w != sw {
                    patch(&mut d, w, stored_h);
                    return Ok(out(
                        &d,
                        &format!("按像素数据推断：高 {sh} 不变，真实宽 {w}（原记录 {sw}）。"),
                    ));
                }
            }
        }
        Ok(out(
            &d,
            &format!(
                "无法由文件大小唯一确定尺寸（当前 {sw}×{sh}，位深 {bpp}，像素区 {avail} 字节）。\
                 请用「手动」模式指定宽/高。"
            ),
        ))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "bmp_fix",
            IMG,
            "BMP 宽高修复",
            AMBER,
            vec![req("data", "BMP", PortType::Any)],
            vec![
                req("image", "修复后", PortType::Image),
                opt("bytes", "字节", PortType::Bytes),
                opt("report", "分析", PortType::Text),
            ],
            vec![
                ParamSpec::select("mode", "模式", &["自动", "手动"], "自动"),
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
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    /// 造一个 w×h、给定位深的未压缩 BMP（像素填 0，无调色板）。
    fn make_bmp(w: u32, h: u32, bpp: u32) -> Vec<u8> {
        let rs = ((bpp * w).div_ceil(32) * 4) as usize;
        let off = 54usize; // 14 文件头 + 40 BITMAPINFOHEADER
        let size = off + rs * h as usize;
        let mut d = vec![0u8; size];
        d[0..2].copy_from_slice(b"BM");
        d[2..6].copy_from_slice(&(size as u32).to_le_bytes());
        d[10..14].copy_from_slice(&(off as u32).to_le_bytes());
        d[14..18].copy_from_slice(&40u32.to_le_bytes());
        d[18..22].copy_from_slice(&(w as i32).to_le_bytes());
        d[22..26].copy_from_slice(&(h as i32).to_le_bytes());
        d[26..28].copy_from_slice(&1u16.to_le_bytes());
        d[28..30].copy_from_slice(&(bpp as u16).to_le_bytes());
        d
    }

    fn fix(bmp: Vec<u8>, params: serde_json::Value) -> Vec<u8> {
        let mut inputs = PortMap::new();
        inputs.insert(
            "data".into(),
            PortValue::Bytes(Arc::from(bmp.into_boxed_slice())),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "bmp_fix",
            &inputs,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("bytes") {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("{o:?}"),
        }
    }

    fn dims(d: &[u8]) -> (i32, i32) {
        (u32le(d, 0x12) as i32, u32le(d, 0x16) as i32)
    }

    #[test]
    fn recovers_shrunk_height() {
        let mut bmp = make_bmp(40, 30, 24);
        bmp[0x16..0x1A].copy_from_slice(&5i32.to_le_bytes()); // 假高度 5
        let fixed = fix(bmp, serde_json::json!({"mode":"自动"}));
        assert_eq!(dims(&fixed), (40, 30));
    }

    #[test]
    fn recovers_scrambled_width() {
        // 32bpp：行长 = 4*w，严格递增，宽度反推无歧义。
        let mut bmp = make_bmp(50, 12, 32);
        bmp[0x12..0x16].copy_from_slice(&999i32.to_le_bytes()); // 假宽度 999
        let fixed = fix(bmp, serde_json::json!({"mode":"自动"}));
        assert_eq!(dims(&fixed), (50, 12));
    }

    #[test]
    fn manual_sets_dims() {
        let bmp = make_bmp(8, 8, 24);
        let fixed = fix(bmp, serde_json::json!({"mode":"手动","width":16,"height":9}));
        assert_eq!(dims(&fixed), (16, 9));
    }
}
