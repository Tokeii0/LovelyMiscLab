//! Repair a JPEG whose SOF width/height were tampered with — the JPEG twin of
//! the PNG height trick (shrink the declared height so a viewer crops the bottom
//! and hides content). JPEG has no CRC on the SOF, but for baseline images the
//! true height is recoverable: entropy-decode the scan just enough to *count*
//! MCUs, then `true_height = ceil(mcu_rows) × mcu_height` (the width is intact).
//! A manual mode simply forces given dimensions (for progressive JPEGs, where
//! MCU counting across scans isn't supported, or to reveal by over-sizing).
use super::image_util::{data_url, input_bytes, to_png};
use super::prelude::*;

struct Component {
    h: u8,
    v: u8,
}
struct ScanComp {
    comp: usize, // index into components
    td: u8,
    ta: u8,
}
struct Sof {
    off: usize, // offset of the SOF marker (dims live at off+5 / off+7)
    height: u16,
    width: u16,
    comps: Vec<Component>,
    baseline: bool, // SOF0/SOF1 → countable
}

/// Canonical Huffman table (JPEG Annex C), decoded via the maxcode/valptr method.
struct HuffTable {
    mincode: [i32; 17],
    maxcode: [i32; 17],
    valptr: [usize; 17],
    symbols: Vec<u8>,
}

impl HuffTable {
    fn build(counts: &[u8; 17], symbols: Vec<u8>) -> Self {
        let mut mincode = [0i32; 17];
        let mut maxcode = [-1i32; 17];
        let mut valptr = [0usize; 17];
        let mut code = 0i32;
        let mut k = 0usize;
        for l in 1..=16 {
            if counts[l] > 0 {
                valptr[l] = k;
                mincode[l] = code;
                code += counts[l] as i32;
                maxcode[l] = code - 1;
                k += counts[l] as usize;
            }
            code <<= 1;
        }
        HuffTable {
            mincode,
            maxcode,
            valptr,
            symbols,
        }
    }

    fn decode(&self, br: &mut BitReader) -> Option<u8> {
        let mut code = 0i32;
        for l in 1..=16 {
            code = (code << 1) | br.read_bit()? as i32;
            if self.maxcode[l] >= 0 && code <= self.maxcode[l] {
                let idx = self.valptr[l] + (code - self.mincode[l]) as usize;
                return self.symbols.get(idx).copied();
            }
        }
        None
    }
}

/// MSB-first bit reader over the entropy-coded scan, with FF00 unstuffing.
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    cur: u8,
    bits_left: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8], start: usize) -> Self {
        BitReader {
            data,
            pos: start,
            cur: 0,
            bits_left: 0,
        }
    }

    // Fetch the next entropy byte; None at a real marker (0xFFxx, xx != 0x00).
    fn next_byte(&mut self) -> Option<u8> {
        if self.pos >= self.data.len() {
            return None;
        }
        let b = self.data[self.pos];
        if b == 0xFF {
            let n = *self.data.get(self.pos + 1)?;
            if n == 0x00 {
                self.pos += 2; // stuffed 0xFF
                return Some(0xFF);
            }
            return None; // marker — leave pos at the 0xFF
        }
        self.pos += 1;
        Some(b)
    }

    fn read_bit(&mut self) -> Option<u8> {
        if self.bits_left == 0 {
            self.cur = self.next_byte()?;
            self.bits_left = 8;
        }
        self.bits_left -= 1;
        Some((self.cur >> self.bits_left) & 1)
    }

    // Consume `n` bits (their value is irrelevant for counting).
    fn skip(&mut self, n: u8) -> Option<()> {
        for _ in 0..n {
            self.read_bit()?;
        }
        Some(())
    }

    // At a restart boundary: drop padding bits and step past the FFDn marker.
    fn restart(&mut self) -> bool {
        self.bits_left = 0;
        while self.pos + 1 < self.data.len()
            && !(self.data[self.pos] == 0xFF
                && (0xD0..=0xD7).contains(&self.data[self.pos + 1]))
        {
            self.pos += 1;
        }
        if self.pos + 1 < self.data.len() {
            self.pos += 2;
            true
        } else {
            false
        }
    }
}

const SOF_MARKERS: [u8; 13] = [
    0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF,
];

struct Parsed {
    sof: Sof,
    dht: Vec<((u8, u8), HuffTable)>, // (class, id) → table
    dri: u16,
    scan_comps: Vec<ScanComp>,
    scan_start: usize,
}

fn parse(jpeg: &[u8]) -> Result<Parsed, CoreError> {
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return Err(CoreError::Parse("不是有效的 JPEG（缺少 SOI）。".into()));
    }
    let mut o = 2usize;
    let mut sof: Option<Sof> = None;
    let mut dht: Vec<((u8, u8), HuffTable)> = Vec::new();
    let mut dri = 0u16;
    while o + 1 < jpeg.len() {
        if jpeg[o] != 0xFF {
            o += 1;
            continue;
        }
        let m = jpeg[o + 1];
        if m == 0xFF {
            o += 1;
            continue;
        }
        if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) {
            o += 2;
            continue;
        }
        if m == 0xD9 {
            break;
        }
        if o + 4 > jpeg.len() {
            break;
        }
        let len = u16::from_be_bytes([jpeg[o + 2], jpeg[o + 3]]) as usize;
        let seg_end = o + 2 + len;
        if seg_end > jpeg.len() {
            break;
        }
        if SOF_MARKERS.contains(&m) {
            let nc = jpeg[o + 9] as usize;
            let mut comps = Vec::with_capacity(nc);
            for i in 0..nc {
                let c = o + 10 + i * 3;
                comps.push(Component {
                    h: jpeg[c + 1] >> 4,
                    v: jpeg[c + 1] & 0xF,
                });
            }
            sof = Some(Sof {
                off: o,
                height: u16::from_be_bytes([jpeg[o + 5], jpeg[o + 6]]),
                width: u16::from_be_bytes([jpeg[o + 7], jpeg[o + 8]]),
                comps,
                baseline: m == 0xC0 || m == 0xC1,
            });
        } else if m == 0xC4 {
            let mut p = o + 4;
            while p < seg_end {
                let tc = jpeg[p] >> 4;
                let th = jpeg[p] & 0xF;
                let mut counts = [0u8; 17];
                let mut tot = 0usize;
                for i in 1..=16 {
                    counts[i] = jpeg[p + i];
                    tot += counts[i] as usize;
                }
                let syms = jpeg[p + 17..p + 17 + tot].to_vec();
                dht.push(((tc, th), HuffTable::build(&counts, syms)));
                p += 17 + tot;
            }
        } else if m == 0xDD {
            dri = u16::from_be_bytes([jpeg[o + 4], jpeg[o + 5]]);
        } else if m == 0xDA {
            let ns = jpeg[o + 4] as usize;
            let sof = sof.ok_or_else(|| CoreError::Parse("SOS 出现在 SOF 之前。".into()))?;
            let mut scan_comps = Vec::with_capacity(ns);
            for i in 0..ns {
                let c = o + 5 + i * 2;
                let id = jpeg[c];
                // component order in SOS follows SOF order; map by position.
                let comp = (id as usize).saturating_sub(1).min(sof.comps.len() - 1);
                scan_comps.push(ScanComp {
                    comp,
                    td: jpeg[c + 1] >> 4,
                    ta: jpeg[c + 1] & 0xF,
                });
            }
            return Ok(Parsed {
                sof,
                dht,
                dri,
                scan_comps,
                scan_start: seg_end,
            });
        }
        o = seg_end;
    }
    Err(CoreError::Parse("未找到扫描数据（SOS）。".into()))
}

/// Count MCUs in a baseline scan → number of full MCU rows' worth of data.
fn count_mcus(jpeg: &[u8], p: &Parsed) -> Option<usize> {
    let table = |class: u8, id: u8| p.dht.iter().find(|(k, _)| *k == (class, id)).map(|(_, t)| t);
    let mut br = BitReader::new(jpeg, p.scan_start);
    let mut mcu = 0usize;
    let mut since_rst = 0u16;
    let hmax = p.sof.comps.iter().map(|c| c.h).max().unwrap_or(1) as usize;
    let vmax = p.sof.comps.iter().map(|c| c.v).max().unwrap_or(1) as usize;
    let mcu_w = 8 * hmax;
    let mcus_per_row = (p.sof.width as usize).div_ceil(mcu_w.max(1));
    if mcus_per_row == 0 {
        return None;
    }
    let safety = mcus_per_row * 200_000;
    'outer: loop {
        for sc in &p.scan_comps {
            let comp = &p.sof.comps[sc.comp];
            let dc = table(0, sc.td)?;
            let ac = table(1, sc.ta)?;
            for _ in 0..(comp.h as usize * comp.v as usize) {
                // DC: one symbol S → S magnitude bits.
                let s = match dc.decode(&mut br) {
                    Some(v) => v,
                    None => break 'outer,
                };
                if s > 0 && br.skip(s).is_none() {
                    break 'outer;
                }
                // AC: run/size symbols up to 63 coefficients.
                let mut k = 1;
                while k < 64 {
                    let rs = match ac.decode(&mut br) {
                        Some(v) => v,
                        None => break 'outer,
                    };
                    let r = rs >> 4;
                    let ss = rs & 0xF;
                    if ss == 0 {
                        if r == 15 {
                            k += 16; // ZRL
                            continue;
                        }
                        break; // EOB
                    }
                    k += r as usize;
                    if br.skip(ss).is_none() {
                        break 'outer;
                    }
                    k += 1;
                }
            }
        }
        mcu += 1;
        since_rst += 1;
        if p.dri > 0 && since_rst == p.dri {
            since_rst = 0;
            if !br.restart() {
                break;
            }
        }
        if mcu > safety {
            break;
        }
    }
    if mcu == 0 {
        return None;
    }
    let mcu_rows = mcu.div_ceil(mcus_per_row);
    Some(mcu_rows * 8 * vmax)
}

fn out(bytes: &[u8], report: &str) -> PortMap {
    let mut m = PortMap::new();
    // Best-effort re-render so the fixed image previews on the node.
    if let Ok(img) = image::load_from_memory(bytes) {
        if let Ok(png) = to_png(&img.to_rgba8()) {
            m.insert("image".into(), PortValue::Image(data_url(&png, "image/png")));
        }
    }
    m.insert(
        "bytes".into(),
        PortValue::Bytes(Arc::from(bytes.to_vec().into_boxed_slice())),
    );
    m.insert("report".into(), PortValue::Text(report.to_string()));
    m
}

fn set_dims(data: &mut [u8], sof_off: usize, w: u16, h: u16) {
    data[sof_off + 5..sof_off + 7].copy_from_slice(&h.to_be_bytes());
    data[sof_off + 7..sof_off + 9].copy_from_slice(&w.to_be_bytes());
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
        let parsed = parse(&data)?;
        let (sw, sh, off) = (parsed.sof.width, parsed.sof.height, parsed.sof.off);

        if pstr(p, "mode", "自动") == "手动" {
            let w = pnum(p, "width", 0.0) as u16;
            let h = pnum(p, "height", 0.0) as u16;
            let w = if w == 0 { sw } else { w };
            let h = if h == 0 { sh } else { h };
            set_dims(&mut data, off, w, h);
            return Ok(out(&data, &format!("手动设置为 {w}×{h}（原 {sw}×{sh}）。")));
        }

        // 自动：baseline 才能数 MCU 恢复高度。
        if !parsed.sof.baseline {
            return Ok(out(
                &data,
                &format!(
                    "当前是渐进式/非基线 JPEG（{sw}×{sh}），自动数 MCU 不支持。\
                     请切到「手动」模式，把高度调大以显示隐藏内容。"
                ),
            ));
        }
        match count_mcus(&data, &parsed) {
            Some(true_h) if true_h as u16 as usize == true_h && true_h != sh as usize => {
                let h = true_h as u16;
                set_dims(&mut data, off, sw, h);
                Ok(out(
                    &data,
                    &format!("按扫描数据恢复：真实高度 {h}（原记录 {sh}，宽度 {sw} 不变）。"),
                ))
            }
            Some(true_h) if true_h == sh as usize => Ok(out(
                &data,
                &format!("高度 {sh} 与扫描数据一致（{sw}×{sh}），无需修复。"),
            )),
            _ => Ok(out(
                &data,
                &format!(
                    "无法从扫描数据恢复高度（{sw}×{sh}）。可能有重启标记异常或非标准编码，\
                     请用「手动」模式指定高度。"
                ),
            )),
        }
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "jpg_fix",
            IMG,
            "JPG 宽高修复",
            AMBER,
            vec![req("data", "JPEG", PortType::Any)],
            vec![
                req("image", "修复后", PortType::Image),
                opt("bytes", "字节", PortType::Bytes),
                opt("report", "分析", PortType::Text),
            ],
            vec![
                ParamSpec::select("mode", "模式", &["自动", "手动"], "自动"),
                ParamSpec::number("width", "宽(手动,0=不改)", 0.0, 100_000.0, 1.0, 0.0),
                ParamSpec::number("height", "高(手动,0=不改)", 0.0, 100_000.0, 1.0, 0.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a tiny baseline JPEG (via the image crate), then shrink its SOF
    // height without touching the scan — exactly the CTF tamper.
    fn tampered_jpeg(w: u32, h: u32, fake_h: u16) -> (Vec<u8>, usize) {
        use image::codecs::jpeg::JpegEncoder;
        use image::{ExtendedColorType, RgbImage};
        let mut img = RgbImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = image::Rgb([(x * 7) as u8, (y * 5) as u8, ((x + y) * 3) as u8]);
        }
        let mut jpg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpg, 90)
            .encode(img.as_raw(), w, h, ExtendedColorType::Rgb8)
            .unwrap();
        let off = parse(&jpg).unwrap().sof.off;
        jpg[off + 5..off + 7].copy_from_slice(&fake_h.to_be_bytes());
        (jpg, off)
    }

    #[test]
    fn recovers_true_height() {
        // 128×80 image, MCU height 8 (or 16 with subsampling); tamper 80 → 8.
        let (jpg, off) = tampered_jpeg(128, 80, 8);
        let p = parse(&jpg).unwrap();
        assert!(p.sof.baseline);
        let recovered = count_mcus(&jpg, &p).expect("count");
        // recovered is rounded up to a full MCU row; must cover the real 80 rows.
        assert!(recovered >= 80, "recovered {recovered}");
        assert!(recovered < 80 + 16, "recovered {recovered} too large");
        let _ = off;
    }

    #[test]
    fn parses_dimensions() {
        let (jpg, _) = tampered_jpeg(64, 48, 48);
        let p = parse(&jpg).unwrap();
        assert_eq!(p.sof.width, 64);
    }
}
