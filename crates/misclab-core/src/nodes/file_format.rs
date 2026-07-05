//! File-format forensics nodes: lightweight structural inspectors for common
//! CTF/forensics containers. These complement `detect_file_type` and
//! `file_carve`: they do not decode pixels or extract files; they expose the
//! format-level blocks that often carry hidden comments, appended data, or
//! intentionally inconsistent metadata.

use crc32fast::Hasher;
use serde_json::json;

use super::prelude::*;

fn be16(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}

fn be32(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn ascii_preview(data: &[u8], limit: usize) -> String {
    data.iter()
        .take(limit)
        .map(|&b| match b {
            b'\t' | b'\n' | b'\r' => ' ',
            0x20..=0x7e => b as char,
            _ => '.',
        })
        .collect::<String>()
        .trim()
        .to_string()
}

// ---------------------------------------------------------------- PNG chunks

struct PngChunks;

impl Node for PngChunks {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
        if data.len() < SIG.len() || &data[..8] != SIG {
            return Err(CoreError::Parse("不是 PNG 文件（缺少 PNG 签名）".into()));
        }

        let mut pos = 8usize;
        let mut rows = Vec::new();
        let mut arr = Vec::new();
        let mut names = Vec::new();
        while pos + 12 <= data.len() {
            let offset = pos;
            let len = be32(&data, pos) as usize;
            let name_bytes = &data[pos + 4..pos + 8];
            let name = String::from_utf8_lossy(name_bytes).to_string();
            let payload_start = pos + 8;
            let payload_end = payload_start.saturating_add(len);
            let crc_off = payload_end;
            if crc_off + 4 > data.len() {
                return Err(CoreError::Parse(format!(
                    "PNG chunk {name} @ 0x{offset:x} 超出文件边界"
                )));
            }
            let stored_crc = be32(&data, crc_off);
            let mut h = Hasher::new();
            h.update(name_bytes);
            h.update(&data[payload_start..payload_end]);
            let calc_crc = h.finalize();
            let crc_ok = stored_crc == calc_crc;
            let ancillary = name_bytes[0].is_ascii_lowercase();
            let private = name_bytes[1].is_ascii_lowercase();
            let reserved_ok = name_bytes[2].is_ascii_uppercase();
            let safe_to_copy = name_bytes[3].is_ascii_lowercase();
            let preview = if matches!(name.as_str(), "tEXt" | "iTXt" | "zTXt") {
                ascii_preview(&data[payload_start..payload_end], 80)
            } else {
                String::new()
            };

            names.push(name.clone());
            rows.push(format!(
                "0x{offset:08x}  {name:<4}  {len:>8} bytes  {}{}{}{}  CRC {}",
                if ancillary { "anc" } else { "crit" },
                if private { "/priv" } else { "" },
                if reserved_ok { "" } else { "/reserved!" },
                if safe_to_copy { "/safe" } else { "" },
                if crc_ok { "ok" } else { "BAD" },
            ));
            arr.push(json!({
                "offset": offset,
                "type": name,
                "length": len,
                "crcStored": stored_crc,
                "crcCalculated": calc_crc,
                "crcOk": crc_ok,
                "ancillary": ancillary,
                "private": private,
                "reservedOk": reserved_ok,
                "safeToCopy": safe_to_copy,
                "preview": preview,
            }));

            pos = crc_off + 4;
            if &name_bytes == b"IEND" {
                break;
            }
        }
        if pos < data.len() {
            rows.push(format!(
                "0x{pos:08x}  trailing  {} bytes after IEND/last chunk",
                data.len() - pos
            ));
            arr.push(json!({ "offset": pos, "type": "trailing", "length": data.len() - pos }));
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("chunks".into(), PortValue::StringList(names));
        Ok(m)
    }
}

// ---------------------------------------------------------------- JPEG markers

fn jpeg_marker_name(code: u8) -> String {
    match code {
        0xd8 => "SOI".into(),
        0xd9 => "EOI".into(),
        0xda => "SOS".into(),
        0xdb => "DQT".into(),
        0xc4 => "DHT".into(),
        0xdd => "DRI".into(),
        0xfe => "COM".into(),
        0xc0 => "SOF0".into(),
        0xc1 => "SOF1".into(),
        0xc2 => "SOF2".into(),
        0xc3 => "SOF3".into(),
        0xc5 => "SOF5".into(),
        0xc6 => "SOF6".into(),
        0xc7 => "SOF7".into(),
        0xc9 => "SOF9".into(),
        0xca => "SOF10".into(),
        0xcb => "SOF11".into(),
        0xcd => "SOF13".into(),
        0xce => "SOF14".into(),
        0xcf => "SOF15".into(),
        0xe0..=0xef => format!("APP{}", code - 0xe0),
        0xd0..=0xd7 => format!("RST{}", code - 0xd0),
        0x01 => "TEM".into(),
        _ => format!("0x{code:02x}"),
    }
}

fn jpeg_standalone(code: u8) -> bool {
    matches!(code, 0xd8 | 0xd9 | 0xd0..=0xd7 | 0x01)
}

struct JpegMarkers;

impl Node for JpegMarkers {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        if data.len() < 2 || data[0] != 0xff || data[1] != 0xd8 {
            return Err(CoreError::Parse("不是 JPEG 文件（缺少 SOI 标记）".into()));
        }

        let mut pos = 0usize;
        let mut rows = Vec::new();
        let mut arr = Vec::new();
        let mut markers = Vec::new();
        let mut ended_at = None;

        while pos + 1 < data.len() {
            while pos < data.len() && data[pos] != 0xff {
                pos += 1;
            }
            if pos + 1 >= data.len() {
                break;
            }
            let marker_offset = pos;
            while pos < data.len() && data[pos] == 0xff {
                pos += 1;
            }
            if pos >= data.len() {
                break;
            }
            let code = data[pos];
            pos += 1;
            if code == 0x00 {
                continue;
            }
            let name = jpeg_marker_name(code);
            markers.push(name.clone());
            if jpeg_standalone(code) {
                rows.push(format!("0x{marker_offset:08x}  FF{code:02X}  {name:<6}"));
                arr.push(json!({ "offset": marker_offset, "marker": format!("FF{code:02X}"), "name": name, "payloadLength": 0 }));
                if code == 0xd9 {
                    ended_at = Some(pos);
                    break;
                }
                continue;
            }
            if pos + 2 > data.len() {
                return Err(CoreError::Parse(format!(
                    "JPEG marker {name} @ 0x{marker_offset:x} 缺少长度字段"
                )));
            }
            let seg_len = be16(&data, pos) as usize;
            if seg_len < 2 || pos + seg_len > data.len() {
                return Err(CoreError::Parse(format!(
                    "JPEG marker {name} @ 0x{marker_offset:x} 长度无效"
                )));
            }
            let payload_start = pos + 2;
            let payload_len = seg_len - 2;
            let payload = &data[payload_start..payload_start + payload_len];
            let preview = if matches!(code, 0xe0..=0xef | 0xfe) {
                ascii_preview(payload, 80)
            } else {
                String::new()
            };
            rows.push(format!(
                "0x{marker_offset:08x}  FF{code:02X}  {name:<6}  {payload_len:>8} bytes{}",
                if preview.is_empty() {
                    String::new()
                } else {
                    format!("  {preview}")
                }
            ));
            arr.push(json!({
                "offset": marker_offset,
                "marker": format!("FF{code:02X}"),
                "name": name,
                "segmentLength": seg_len,
                "payloadLength": payload_len,
                "preview": preview,
            }));
            pos += seg_len;
            if code == 0xda {
                // SOS starts entropy-coded scan data. Find EOI while respecting
                // byte-stuffed FF00 and restart markers.
                let scan_start = pos;
                let mut p = pos;
                while p + 1 < data.len() {
                    if data[p] == 0xff {
                        let next = data[p + 1];
                        if next == 0x00 || (0xd0..=0xd7).contains(&next) {
                            p += 2;
                            continue;
                        }
                        if next == 0xd9 {
                            if p > scan_start {
                                rows.push(format!(
                                    "0x{scan_start:08x}  scan    {:>8} bytes",
                                    p - scan_start
                                ));
                                arr.push(json!({ "offset": scan_start, "name": "scan", "payloadLength": p - scan_start }));
                            }
                            rows.push(format!("0x{p:08x}  FFD9  EOI"));
                            arr.push(json!({ "offset": p, "marker": "FFD9", "name": "EOI", "payloadLength": 0 }));
                            markers.push("EOI".into());
                            ended_at = Some(p + 2);
                            break;
                        }
                    }
                    p += 1;
                }
                break;
            }
        }
        if let Some(end) = ended_at {
            if end < data.len() {
                rows.push(format!(
                    "0x{end:08x}  trailing  {} bytes after EOI",
                    data.len() - end
                ));
                arr.push(
                    json!({ "offset": end, "name": "trailing", "payloadLength": data.len() - end }),
                );
            }
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("markers".into(), PortValue::StringList(markers));
        Ok(m)
    }
}

// ---------------------------------------------------------------- GIF blocks

fn gif_ext_name(label: u8) -> &'static str {
    match label {
        0xf9 => "Graphic Control Extension",
        0xfe => "Comment Extension",
        0xff => "Application Extension",
        0x01 => "Plain Text Extension",
        _ => "Unknown Extension",
    }
}

fn read_gif_subblocks(
    data: &[u8],
    mut pos: usize,
) -> Result<(usize, usize, usize, String), CoreError> {
    let mut blocks = 0usize;
    let mut payload = 0usize;
    let mut preview = Vec::new();
    loop {
        if pos >= data.len() {
            return Err(CoreError::Parse("GIF 子块未正常终止".into()));
        }
        let len = data[pos] as usize;
        pos += 1;
        if len == 0 {
            break;
        }
        if pos + len > data.len() {
            return Err(CoreError::Parse("GIF 子块越界".into()));
        }
        blocks += 1;
        payload += len;
        if preview.len() < 80 {
            let take = (80 - preview.len()).min(len);
            preview.extend_from_slice(&data[pos..pos + take]);
        }
        pos += len;
    }
    Ok((pos, blocks, payload, ascii_preview(&preview, 80)))
}

struct GifBlocks;

impl Node for GifBlocks {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        if data.len() < 13 || (&data[..6] != b"GIF87a" && &data[..6] != b"GIF89a") {
            return Err(CoreError::Parse(
                "不是 GIF 文件（缺少 GIF87a/GIF89a 头）".into(),
            ));
        }
        let width = le16(&data, 6);
        let height = le16(&data, 8);
        let packed = data[10];
        let gct = packed & 0x80 != 0;
        let gct_size = if gct {
            3usize * (1usize << ((packed & 0x07) + 1))
        } else {
            0
        };
        let mut pos = 13usize + gct_size;
        if pos > data.len() {
            return Err(CoreError::Parse("GIF 全局颜色表越界".into()));
        }

        let mut rows = vec![format!(
            "0x00000000  Header  {}  {}x{}  GCT {} bytes",
            String::from_utf8_lossy(&data[..6]),
            width,
            height,
            gct_size
        )];
        let mut arr = vec![json!({
            "offset": 0,
            "type": "header",
            "version": String::from_utf8_lossy(&data[..6]).to_string(),
            "width": width,
            "height": height,
            "globalColorTableBytes": gct_size,
        })];
        let mut block_names = vec!["Header".to_string()];

        while pos < data.len() {
            let offset = pos;
            match data[pos] {
                0x2c => {
                    if pos + 10 > data.len() {
                        return Err(CoreError::Parse("GIF 图像描述符越界".into()));
                    }
                    let left = le16(&data, pos + 1);
                    let top = le16(&data, pos + 3);
                    let w = le16(&data, pos + 5);
                    let h = le16(&data, pos + 7);
                    let ipacked = data[pos + 9];
                    let lct = ipacked & 0x80 != 0;
                    let interlace = ipacked & 0x40 != 0;
                    let lct_size = if lct {
                        3usize * (1usize << ((ipacked & 0x07) + 1))
                    } else {
                        0
                    };
                    pos += 10 + lct_size;
                    if pos >= data.len() {
                        return Err(CoreError::Parse("GIF 图像数据缺少 LZW 最小码长".into()));
                    }
                    let lzw_min = data[pos];
                    let (end, blocks, payload, _) = read_gif_subblocks(&data, pos + 1)?;
                    rows.push(format!(
                        "0x{offset:08x}  Image   {w}x{h}+{left}+{top}  LZW {lzw_min}  {blocks} blocks / {payload} bytes{}",
                        if interlace { "  interlace" } else { "" }
                    ));
                    arr.push(json!({
                        "offset": offset,
                        "type": "image",
                        "left": left,
                        "top": top,
                        "width": w,
                        "height": h,
                        "localColorTableBytes": lct_size,
                        "interlace": interlace,
                        "lzwMinCodeSize": lzw_min,
                        "subBlocks": blocks,
                        "payloadBytes": payload,
                    }));
                    block_names.push("Image".into());
                    pos = end;
                }
                0x21 => {
                    if pos + 2 > data.len() {
                        return Err(CoreError::Parse("GIF 扩展块越界".into()));
                    }
                    let label = data[pos + 1];
                    let name = gif_ext_name(label);
                    let (end, blocks, payload, preview) = read_gif_subblocks(&data, pos + 2)?;
                    rows.push(format!(
                        "0x{offset:08x}  Ext {label:02X}  {name:<27}  {blocks} blocks / {payload} bytes{}",
                        if preview.is_empty() { String::new() } else { format!("  {preview}") }
                    ));
                    arr.push(json!({
                        "offset": offset,
                        "type": "extension",
                        "label": label,
                        "name": name,
                        "subBlocks": blocks,
                        "payloadBytes": payload,
                        "preview": preview,
                    }));
                    block_names.push(name.into());
                    pos = end;
                }
                0x3b => {
                    rows.push(format!("0x{offset:08x}  Trailer"));
                    arr.push(json!({ "offset": offset, "type": "trailer" }));
                    block_names.push("Trailer".into());
                    pos += 1;
                    break;
                }
                other => {
                    return Err(CoreError::Parse(format!(
                        "未知 GIF 块 0x{other:02x} @ 0x{offset:x}"
                    )));
                }
            }
        }
        if pos < data.len() {
            rows.push(format!(
                "0x{pos:08x}  trailing  {} bytes after trailer",
                data.len() - pos
            ));
            arr.push(json!({ "offset": pos, "type": "trailing", "length": data.len() - pos }));
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("blocks".into(), PortValue::StringList(block_names));
        Ok(m)
    }
}

// ---------------------------------------------------------------- ZIP local vs central directory

#[derive(Clone)]
struct ZipCentral {
    name: String,
    flags: u16,
    method: u16,
    crc: u32,
    csize: u32,
    usize_: u32,
    local_offset: u32,
}

#[derive(Clone)]
struct ZipLocal {
    name: String,
    flags: u16,
    method: u16,
    crc: u32,
    csize: u32,
    usize_: u32,
}

fn find_eocd(d: &[u8]) -> Option<usize> {
    if d.len() < 22 {
        return None;
    }
    let start = d.len().saturating_sub(22 + 65535);
    (start..=d.len() - 22)
        .rev()
        .find(|&i| &d[i..i + 4] == b"PK\x05\x06")
}

fn parse_zip_central(data: &[u8]) -> Result<Vec<ZipCentral>, CoreError> {
    let eocd = find_eocd(data).ok_or_else(|| CoreError::Parse("找不到 ZIP EOCD".into()))?;
    let count = le16(data, eocd + 10) as usize;
    let mut pos = le32(data, eocd + 16) as usize;
    let mut out = Vec::new();
    for idx in 0..count {
        if pos + 46 > data.len() || &data[pos..pos + 4] != b"PK\x01\x02" {
            return Err(CoreError::Parse(format!("中央目录第 {idx} 条解析失败")));
        }
        let n = le16(data, pos + 28) as usize;
        let m = le16(data, pos + 30) as usize;
        let k = le16(data, pos + 32) as usize;
        let name_start = pos + 46;
        let next = name_start + n + m + k;
        if next > data.len() {
            return Err(CoreError::Parse("中央目录条目越界".into()));
        }
        out.push(ZipCentral {
            name: String::from_utf8_lossy(&data[name_start..name_start + n]).to_string(),
            flags: le16(data, pos + 8),
            method: le16(data, pos + 10),
            crc: le32(data, pos + 16),
            csize: le32(data, pos + 20),
            usize_: le32(data, pos + 24),
            local_offset: le32(data, pos + 42),
        });
        pos = next;
    }
    Ok(out)
}

fn parse_zip_local(data: &[u8], off: usize) -> Result<ZipLocal, CoreError> {
    if off + 30 > data.len() || &data[off..off + 4] != b"PK\x03\x04" {
        return Err(CoreError::Parse(format!("本地文件头 0x{off:x} 无效")));
    }
    let n = le16(data, off + 26) as usize;
    let m = le16(data, off + 28) as usize;
    let name_start = off + 30;
    if name_start + n + m > data.len() {
        return Err(CoreError::Parse(format!("本地文件头 0x{off:x} 越界")));
    }
    Ok(ZipLocal {
        name: String::from_utf8_lossy(&data[name_start..name_start + n]).to_string(),
        flags: le16(data, off + 6),
        method: le16(data, off + 8),
        crc: le32(data, off + 14),
        csize: le32(data, off + 18),
        usize_: le32(data, off + 22),
    })
}

struct ZipDirectoryDiff;

impl Node for ZipDirectoryDiff {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "archive")?;
        if !(data.starts_with(b"PK\x03\x04")
            || data.starts_with(b"PK\x05\x06")
            || data.starts_with(b"PK\x01\x02"))
        {
            return Err(CoreError::Parse("不是 ZIP 文件（缺少 PK 头）".into()));
        }
        let central = parse_zip_central(&data)?;
        let mut rows = Vec::new();
        let mut arr = Vec::new();
        let mut mismatch = false;
        for (idx, c) in central.iter().enumerate() {
            let l = parse_zip_local(&data, c.local_offset as usize)?;
            let mut diffs = Vec::new();
            if l.name != c.name {
                diffs.push(format!("name local='{}' central='{}'", l.name, c.name));
            }
            if l.flags != c.flags {
                diffs.push(format!(
                    "flags local=0x{:04x} central=0x{:04x}",
                    l.flags, c.flags
                ));
            }
            if l.method != c.method {
                diffs.push(format!("method local={} central={}", l.method, c.method));
            }
            let has_data_descriptor = (l.flags | c.flags) & 0x0008 != 0;
            if !has_data_descriptor {
                if l.crc != c.crc {
                    diffs.push(format!("crc local={:08x} central={:08x}", l.crc, c.crc));
                }
                if l.csize != c.csize {
                    diffs.push(format!("compressed local={} central={}", l.csize, c.csize));
                }
                if l.usize_ != c.usize_ {
                    diffs.push(format!("size local={} central={}", l.usize_, c.usize_));
                }
            }
            if !diffs.is_empty() {
                mismatch = true;
            }
            rows.push(format!(
                "#{idx:<3} {:<40}  LFH@0x{:08x}  {}",
                c.name.chars().take(40).collect::<String>(),
                c.local_offset,
                if diffs.is_empty() {
                    "OK".into()
                } else {
                    diffs.join("; ")
                },
            ));
            arr.push(json!({
                "index": idx,
                "name": c.name,
                "localOffset": c.local_offset,
                "local": {
                    "name": l.name,
                    "flags": l.flags,
                    "method": l.method,
                    "crc": l.crc,
                    "compressedSize": l.csize,
                    "uncompressedSize": l.usize_,
                },
                "central": {
                    "name": c.name,
                    "flags": c.flags,
                    "method": c.method,
                    "crc": c.crc,
                    "compressedSize": c.csize,
                    "uncompressedSize": c.usize_,
                },
                "diffs": diffs,
            }));
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("mismatch".into(), PortValue::Bool(mismatch));
        m.insert("count".into(), PortValue::Number(central.len() as f64));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "png_chunks",
            UTIL,
            "PNG Chunk 列表",
            AMBER,
            vec![req("data", "PNG", PortType::Any)],
            vec![
                req("text", "Chunk 表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("chunks", "类型列表", PortType::StringList),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(PngChunks)),
    );
    reg.register(
        desc(
            "jpeg_markers",
            UTIL,
            "JPEG Marker 列表",
            AMBER,
            vec![req("data", "JPEG", PortType::Any)],
            vec![
                req("text", "Marker 表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("markers", "标记列表", PortType::StringList),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(JpegMarkers)),
    );
    reg.register(
        desc(
            "gif_blocks",
            UTIL,
            "GIF Block 列表",
            AMBER,
            vec![req("data", "GIF", PortType::Any)],
            vec![
                req("text", "Block 表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("blocks", "块列表", PortType::StringList),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(GifBlocks)),
    );
    reg.register(
        desc(
            "zip_directory_diff",
            ARC,
            "ZIP 目录差异",
            AMBER,
            vec![req("archive", "ZIP", PortType::Any)],
            vec![
                req("text", "差异报告", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("mismatch", "存在差异", PortType::Bool),
                opt("count", "条目数", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(ZipDirectoryDiff)),
    );
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;
    use zip::write::SimpleFileOptions;

    fn run_bytes(id: &str, port: &str, data: Vec<u8>) -> PortMap {
        let mut i = PortMap::new();
        i.insert(
            port.into(),
            PortValue::Bytes(Arc::from(data.into_boxed_slice())),
        );
        GraphExecutor::run_node(
            &default_registry(),
            id,
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    fn png_chunk(name: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(payload);
        let mut h = Hasher::new();
        h.update(name);
        h.update(payload);
        out.extend_from_slice(&h.finalize().to_be_bytes());
        out
    }

    #[test]
    fn png_chunks_reports_crc() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(png_chunk(b"IHDR", &[]));
        png.extend(png_chunk(b"tEXt", b"Comment\0flag"));
        png.extend(png_chunk(b"IEND", &[]));
        let out = run_bytes("png_chunks", "data", png);
        let text = match out.get("text") {
            Some(PortValue::Text(s)) => s,
            o => panic!("{o:?}"),
        };
        assert!(
            text.contains("IHDR") && text.contains("tEXt") && text.contains("CRC ok"),
            "{text}"
        );
        assert!(
            matches!(out.get("chunks"), Some(PortValue::StringList(v)) if v.contains(&"IEND".to_string()))
        );
    }

    #[test]
    fn jpeg_markers_reports_comments() {
        let mut jpg = vec![0xff, 0xd8, 0xff, 0xfe, 0x00, 0x07];
        jpg.extend_from_slice(b"hello");
        jpg.extend_from_slice(&[0xff, 0xd9]);
        let out = run_bytes("jpeg_markers", "data", jpg);
        let text = match out.get("text") {
            Some(PortValue::Text(s)) => s,
            o => panic!("{o:?}"),
        };
        assert!(
            text.contains("SOI") && text.contains("COM") && text.contains("hello"),
            "{text}"
        );
    }

    #[test]
    fn gif_blocks_reports_comment_and_trailer() {
        let mut gif = b"GIF89a\x01\x00\x01\x00\x00\x00\x00".to_vec();
        gif.extend_from_slice(&[0x21, 0xfe, 0x04]);
        gif.extend_from_slice(b"flag");
        gif.extend_from_slice(&[0x00, 0x3b]);
        let out = run_bytes("gif_blocks", "data", gif);
        let text = match out.get("text") {
            Some(PortValue::Text(s)) => s,
            o => panic!("{o:?}"),
        };
        assert!(
            text.contains("Comment Extension") && text.contains("Trailer"),
            "{text}"
        );
    }

    fn plain_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            w.start_file(
                "flag.txt",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            w.write_all(b"flag{zip_dir_diff}").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn zip_directory_diff_finds_flag_mismatch() {
        let mut z = plain_zip();
        let cd = z.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
        let flags = le16(&z, cd + 8) | 1;
        z[cd + 8..cd + 10].copy_from_slice(&flags.to_le_bytes());
        let out = run_bytes("zip_directory_diff", "archive", z);
        let text = match out.get("text") {
            Some(PortValue::Text(s)) => s,
            o => panic!("{o:?}"),
        };
        assert!(text.contains("flags"), "{text}");
        assert!(matches!(out.get("mismatch"), Some(PortValue::Bool(true))));
    }
}
