//! Document and mobile-package forensics nodes.
//!
//! These are structural inspectors for common CTF/forensics artifacts. They do
//! not try to fully render PDF/Office/APK contents; they expose objects,
//! streams, metadata, embedded entries, manifest strings, and DEX strings so the
//! graph can route them into strings, regex, hexdump, or archive extraction.

use std::io::{Cursor, Read, Seek};

use flate2::read::ZlibDecoder;
use regex::{bytes::Regex as BytesRegex, Regex};
use serde_json::json;

use super::prelude::*;

fn le16(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(o)?, *d.get(o + 1)?]))
}

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *d.get(o)?,
        *d.get(o + 1)?,
        *d.get(o + 2)?,
        *d.get(o + 3)?,
    ]))
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

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_ascii_u32(bytes: &[u8]) -> Option<u32> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

// ---------------------------------------------------------------- PDF objects

#[derive(Debug, Clone)]
struct PdfStreamInfo {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
struct PdfObject {
    number: u32,
    generation: u32,
    offset: usize,
    content_start: usize,
    content_end: usize,
    typ: Option<String>,
    subtype: Option<String>,
    filter: Option<String>,
    stream: Option<PdfStreamInfo>,
}

fn pdf_header_offset(data: &[u8]) -> Option<usize> {
    let scan = data.len().min(1024);
    find_bytes(&data[..scan], b"%PDF-")
}

fn pdf_name_value(content: &[u8], key: &str) -> Option<String> {
    let needle = format!("/{key}");
    let mut pos = find_bytes(content, needle.as_bytes())? + needle.len();
    while pos < content.len() && content[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos < content.len() && content[pos] == b'[' {
        pos += 1;
        while pos < content.len() && content[pos].is_ascii_whitespace() {
            pos += 1;
        }
    }
    if pos < content.len() && content[pos] == b'/' {
        pos += 1;
    }
    let start = pos;
    while pos < content.len()
        && !content[pos].is_ascii_whitespace()
        && !matches!(content[pos], b'/' | b'<' | b'>' | b'[' | b']' | b'(' | b')')
    {
        pos += 1;
    }
    (pos > start).then(|| String::from_utf8_lossy(&content[start..pos]).to_string())
}

fn find_pdf_stream(content: &[u8], base: usize) -> Option<PdfStreamInfo> {
    let mut search = 0usize;
    while let Some(rel) = find_bytes(&content[search..], b"stream") {
        let s = search + rel;
        let before_ok = s == 0
            || matches!(
                content[s.saturating_sub(1)],
                b'\n' | b'\r' | b' ' | b'\t' | b'>'
            );
        let mut payload_start = s + 6;
        if !before_ok || payload_start >= content.len() {
            search = s + 6;
            continue;
        }
        match content[payload_start] {
            b'\r' => {
                payload_start += 1;
                if payload_start < content.len() && content[payload_start] == b'\n' {
                    payload_start += 1;
                }
            }
            b'\n' => payload_start += 1,
            _ => {
                search = s + 6;
                continue;
            }
        }

        let end_rel = find_bytes(&content[payload_start..], b"endstream")?;
        let mut payload_end = payload_start + end_rel;
        if payload_end > payload_start && content[payload_end - 1] == b'\n' {
            payload_end -= 1;
        }
        if payload_end > payload_start && content[payload_end - 1] == b'\r' {
            payload_end -= 1;
        }
        return Some(PdfStreamInfo {
            start: base + payload_start,
            end: base + payload_end,
        });
    }
    None
}

fn parse_pdf_objects(data: &[u8]) -> Result<Vec<PdfObject>, CoreError> {
    if pdf_header_offset(data).is_none() {
        return Err(CoreError::Parse("不是 PDF 文件（未找到 %PDF- 头）".into()));
    }

    let re = BytesRegex::new(r"(?m)(\d{1,10})\s+(\d{1,10})\s+obj\b").unwrap();
    let mut out = Vec::new();
    for cap in re.captures_iter(data) {
        let Some(m) = cap.get(0) else {
            continue;
        };
        let number = cap
            .get(1)
            .and_then(|m| parse_ascii_u32(m.as_bytes()))
            .unwrap_or(0);
        let generation = cap
            .get(2)
            .and_then(|m| parse_ascii_u32(m.as_bytes()))
            .unwrap_or(0);
        let content_start = m.end();
        let end_rel = find_bytes(&data[content_start..], b"endobj")
            .unwrap_or(data.len().saturating_sub(content_start));
        let content_end = content_start + end_rel;
        let content = &data[content_start..content_end];
        out.push(PdfObject {
            number,
            generation,
            offset: m.start(),
            content_start,
            content_end,
            typ: pdf_name_value(content, "Type"),
            subtype: pdf_name_value(content, "Subtype"),
            filter: pdf_name_value(content, "Filter"),
            stream: find_pdf_stream(content, content_start),
        });
    }
    Ok(out)
}

struct PdfObjects;

impl Node for PdfObjects {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let objects = parse_pdf_objects(&data)?;

        let mut rows = Vec::new();
        let mut arr = Vec::new();
        let mut names = Vec::new();
        for obj in &objects {
            let id = format!("{} {}", obj.number, obj.generation);
            names.push(id.clone());
            let len = obj.content_end.saturating_sub(obj.content_start);
            let stream_len = obj.stream.as_ref().map(|s| s.end.saturating_sub(s.start));
            rows.push(format!(
                "obj {id:<9} @ 0x{:08x}  len {:>7}  type={} subtype={} filter={} stream={}",
                obj.offset,
                len,
                obj.typ.as_deref().unwrap_or("-"),
                obj.subtype.as_deref().unwrap_or("-"),
                obj.filter.as_deref().unwrap_or("-"),
                stream_len
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".to_string())
            ));
            arr.push(json!({
                "object": obj.number,
                "generation": obj.generation,
                "offset": obj.offset,
                "contentLength": len,
                "type": obj.typ,
                "subtype": obj.subtype,
                "filter": obj.filter,
                "hasStream": obj.stream.is_some(),
                "streamOffset": obj.stream.as_ref().map(|s| s.start),
                "streamLength": stream_len,
                "preview": ascii_preview(&data[obj.content_start..obj.content_end], 120),
            }));
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("objects".into(), PortValue::StringList(names));
        m.insert("count".into(), PortValue::Number(objects.len() as f64));
        Ok(m)
    }
}

struct PdfStreams;

impl Node for PdfStreams {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let decode_flate = pbool(p, "decodeFlate", true);
        let objects = parse_pdf_objects(&data)?;

        let mut rows = Vec::new();
        let mut arr = Vec::new();
        let mut names = Vec::new();
        let mut first_payload: Option<Vec<u8>> = None;

        for obj in objects.iter().filter(|o| o.stream.is_some()) {
            let stream = obj.stream.as_ref().unwrap();
            let raw = &data[stream.start..stream.end];
            let mut decode_error = None;
            let decoded = if decode_flate && obj.filter.as_deref() == Some("FlateDecode") {
                let mut out = Vec::new();
                match ZlibDecoder::new(raw).read_to_end(&mut out) {
                    Ok(_) => Some(out),
                    Err(e) => {
                        decode_error = Some(e.to_string());
                        None
                    }
                }
            } else {
                None
            };
            let effective = decoded.as_deref().unwrap_or(raw);
            if first_payload.is_none() {
                first_payload = Some(effective.to_vec());
            }

            let id = format!("{} {}", obj.number, obj.generation);
            names.push(id.clone());
            rows.push(format!(
                "obj {id:<9} stream @ 0x{:08x}  raw {:>7}  decoded {:>7}  filter={}  {}",
                stream.start,
                raw.len(),
                decoded.as_ref().map(|d| d.len()).unwrap_or(raw.len()),
                obj.filter.as_deref().unwrap_or("-"),
                ascii_preview(effective, 96)
            ));
            arr.push(json!({
                "object": obj.number,
                "generation": obj.generation,
                "offset": stream.start,
                "rawLength": raw.len(),
                "decodedLength": decoded.as_ref().map(|d| d.len()),
                "filter": obj.filter,
                "decoded": decoded.is_some(),
                "decodeError": decode_error,
                "preview": ascii_preview(effective, 160),
            }));
        }

        let count = names.len();
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("streams".into(), PortValue::StringList(names));
        m.insert("count".into(), PortValue::Number(count as f64));
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(
                first_payload.unwrap_or_default().into_boxed_slice(),
            )),
        );
        Ok(m)
    }
}

// ------------------------------------------------------------- OOXML metadata

fn zip_open(data: Vec<u8>) -> Result<zip::ZipArchive<Cursor<Vec<u8>>>, CoreError> {
    zip::ZipArchive::new(Cursor::new(data)).map_err(|e| CoreError::Parse(format!("zip: {e}")))
}

fn read_zip_entry<R: Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
) -> Result<Option<Vec<u8>>, CoreError> {
    let mut f = match zip.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(CoreError::Parse(format!("读取 {name} 失败: {e}"))),
    };
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(Some(buf))
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn local_xml_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn xml_pairs(source: &str, xml: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let custom_re = Regex::new(
        r#"(?s)<property\b[^>]*\bname="([^"]+)"[^>]*>.*?<([A-Za-z0-9_.:-]+)[^>]*>(.*?)</[A-Za-z0-9_.:-]+>.*?</property>"#,
    )
    .unwrap();
    for cap in custom_re.captures_iter(xml) {
        let name = xml_unescape(cap.get(1).map(|m| m.as_str()).unwrap_or(""));
        let value = xml_unescape(cap.get(3).map(|m| m.as_str()).unwrap_or("").trim());
        if !name.is_empty() && !value.is_empty() {
            out.push((format!("{source}:{name}"), value));
        }
    }

    let tag_re =
        Regex::new(r#"(?s)<([A-Za-z0-9_.:-]+)(?:\s[^>]*)?>([^<>]{1,4096})</[A-Za-z0-9_.:-]+>"#)
            .unwrap();
    for cap in tag_re.captures_iter(xml) {
        let tag = local_xml_name(cap.get(1).map(|m| m.as_str()).unwrap_or(""));
        if tag == "property" {
            continue;
        }
        let value = xml_unescape(cap.get(2).map(|m| m.as_str()).unwrap_or("").trim());
        if !tag.is_empty() && !value.is_empty() {
            out.push((format!("{source}:{tag}"), value));
        }
    }
    out
}

struct OoxmlMetadata;

impl Node for OoxmlMetadata {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "document")?;
        let mut zip = zip_open(data)?;
        let targets = [
            ("core", "docProps/core.xml"),
            ("app", "docProps/app.xml"),
            ("custom", "docProps/custom.xml"),
        ];

        let mut rows = Vec::new();
        let mut arr = Vec::new();
        let mut props = Vec::new();
        for (label, path) in targets {
            let Some(bytes) = read_zip_entry(&mut zip, path)? else {
                continue;
            };
            let xml = String::from_utf8_lossy(&bytes);
            for (name, value) in xml_pairs(label, &xml) {
                rows.push(format!("{name}: {value}"));
                props.push(format!("{name}={value}"));
                arr.push(json!({ "source": path, "name": name, "value": value }));
            }
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("properties".into(), PortValue::StringList(props));
        m.insert("count".into(), PortValue::Number(rows.len() as f64));
        Ok(m)
    }
}

fn ooxml_embedded_kind(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with("/vbaproject.bin") || lower.ends_with("vbaProject.bin") {
        Some("macro")
    } else if lower.contains("/embeddings/") {
        Some("embedding")
    } else if lower.contains("/activex/") {
        Some("activex")
    } else if lower.contains("/oleobject") {
        Some("ole")
    } else if lower.contains("/media/") {
        Some("media")
    } else {
        None
    }
}

struct OoxmlEmbedded;

impl Node for OoxmlEmbedded {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "document")?;
        let mut zip = zip_open(data)?;

        let mut entries = Vec::new();
        let mut files = Vec::new();
        let mut rows = Vec::new();
        let mut has_macro = false;
        for idx in 0..zip.len() {
            let f = zip
                .by_index_raw(idx)
                .map_err(|e| CoreError::Parse(format!("读取 ZIP 条目 {idx} 失败: {e}")))?;
            let name = f.name().to_string();
            let Some(kind) = ooxml_embedded_kind(&name) else {
                continue;
            };
            if kind == "macro" {
                has_macro = true;
            }
            rows.push(format!(
                "{kind:<9} {name}  size={} compressed={} crc={:08x}",
                f.size(),
                f.compressed_size(),
                f.crc32()
            ));
            files.push(name.clone());
            entries.push(json!({
                "name": name,
                "kind": kind,
                "size": f.size(),
                "compressedSize": f.compressed_size(),
                "crc32": f.crc32(),
            }));
        }

        let first = if let Some(name) = files.first() {
            read_zip_entry(&mut zip, name)?.unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(entries)));
        m.insert("files".into(), PortValue::StringList(files.clone()));
        m.insert("count".into(), PortValue::Number(files.len() as f64));
        m.insert("hasMacro".into(), PortValue::Bool(has_macro));
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(first.into_boxed_slice())),
        );
        Ok(m)
    }
}

// ------------------------------------------------------------ APK / AXML

#[derive(Debug, Clone)]
struct AxmlElement {
    name: String,
    attrs: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
struct AxmlReport {
    strings: Vec<String>,
    elements: Vec<AxmlElement>,
}

fn read_res_utf8_len(data: &[u8], pos: usize) -> Option<(usize, usize)> {
    let b0 = *data.get(pos)?;
    if b0 & 0x80 != 0 {
        let b1 = *data.get(pos + 1)?;
        Some(((((b0 & 0x7f) as usize) << 8) | b1 as usize, 2))
    } else {
        Some((b0 as usize, 1))
    }
}

fn read_res_utf16_len(data: &[u8], pos: usize) -> Option<(usize, usize)> {
    let w0 = le16(data, pos)?;
    if w0 & 0x8000 != 0 {
        let w1 = le16(data, pos + 2)?;
        Some(((((w0 & 0x7fff) as usize) << 16) | w1 as usize, 4))
    } else {
        Some((w0 as usize, 2))
    }
}

fn parse_string_pool(data: &[u8], offset: usize) -> Option<Vec<String>> {
    if le16(data, offset)? != 0x0001 {
        return None;
    }
    let header_size = le16(data, offset + 2)? as usize;
    let chunk_size = le32(data, offset + 4)? as usize;
    let string_count = le32(data, offset + 8)? as usize;
    let flags = le32(data, offset + 16)?;
    let strings_start = le32(data, offset + 20)? as usize;
    if offset + chunk_size > data.len() || header_size < 28 {
        return None;
    }

    let utf8 = flags & 0x0000_0100 != 0;
    let offsets_start = offset + header_size;
    let data_start = offset + strings_start;
    let mut strings = Vec::new();
    for i in 0..string_count {
        let item_off = le32(data, offsets_start + i * 4)? as usize;
        let pos = data_start + item_off;
        if pos >= data.len() {
            strings.push(String::new());
            continue;
        }
        let s = if utf8 {
            let (_, a) = read_res_utf8_len(data, pos)?;
            let (byte_len, b) = read_res_utf8_len(data, pos + a)?;
            let start = pos + a + b;
            let end = start.saturating_add(byte_len).min(data.len());
            String::from_utf8_lossy(&data[start..end]).to_string()
        } else {
            let (units, a) = read_res_utf16_len(data, pos)?;
            let start = pos + a;
            let mut words = Vec::new();
            for j in 0..units {
                words.push(le16(data, start + j * 2).unwrap_or(0));
            }
            String::from_utf16_lossy(&words)
        };
        strings.push(s);
    }
    Some(strings)
}

fn axml_string(strings: &[String], idx: u32) -> Option<String> {
    if idx == u32::MAX {
        return None;
    }
    strings.get(idx as usize).cloned()
}

fn axml_value(strings: &[String], raw_idx: u32, data_type: u8, data_value: u32) -> String {
    if let Some(raw) = axml_string(strings, raw_idx) {
        return raw;
    }
    match data_type {
        0x03 => axml_string(strings, data_value).unwrap_or_default(),
        0x10 | 0x11 => data_value.to_string(),
        0x12 => {
            if data_value == 0 {
                "false".into()
            } else {
                "true".into()
            }
        }
        0x01 => format!("@0x{data_value:08x}"),
        0x02 => format!("?0x{data_value:08x}"),
        0x1c..=0x1f => format!("#{:08x}", data_value),
        _ => format!("type=0x{data_type:02x}:0x{data_value:x}"),
    }
}

fn parse_axml(data: &[u8]) -> Result<AxmlReport, CoreError> {
    let root_type = le16(data, 0).ok_or_else(|| CoreError::Parse("AXML 太短".into()))?;
    let start = if root_type == 0x0003 {
        le16(data, 2).unwrap_or(8) as usize
    } else {
        0
    };

    let mut offset = start;
    let mut strings = Vec::new();
    let mut elements = Vec::new();
    while offset + 8 <= data.len() {
        let Some(chunk_type) = le16(data, offset) else {
            break;
        };
        let header_size = le16(data, offset + 2).unwrap_or(8) as usize;
        let chunk_size = le32(data, offset + 4).unwrap_or(0) as usize;
        if chunk_size < 8 || offset + chunk_size > data.len() {
            break;
        }

        match chunk_type {
            0x0001 => {
                if let Some(pool) = parse_string_pool(data, offset) {
                    strings = pool;
                }
            }
            0x0102 => {
                if !strings.is_empty() && chunk_size >= header_size + 20 {
                    let ext = offset + header_size;
                    let name_idx = le32(data, ext + 4).unwrap_or(u32::MAX);
                    let name =
                        axml_string(&strings, name_idx).unwrap_or_else(|| format!("#{name_idx}"));
                    let attr_start = le16(data, ext + 8).unwrap_or(20) as usize;
                    let attr_size = le16(data, ext + 10).unwrap_or(20).max(20) as usize;
                    let attr_count = le16(data, ext + 12).unwrap_or(0) as usize;
                    let attrs_base = ext + attr_start;
                    let mut attrs = Vec::new();
                    for n in 0..attr_count {
                        let a = attrs_base + n * attr_size;
                        if a + 20 > offset + chunk_size {
                            break;
                        }
                        let attr_name_idx = le32(data, a + 4).unwrap_or(u32::MAX);
                        let raw_value_idx = le32(data, a + 8).unwrap_or(u32::MAX);
                        let data_type = *data.get(a + 15).unwrap_or(&0);
                        let data_value = le32(data, a + 16).unwrap_or(0);
                        let attr_name = axml_string(&strings, attr_name_idx)
                            .unwrap_or_else(|| format!("#{attr_name_idx}"));
                        let value = axml_value(&strings, raw_value_idx, data_type, data_value);
                        attrs.push((attr_name, value));
                    }
                    elements.push(AxmlElement { name, attrs });
                }
            }
            _ => {}
        }

        offset += chunk_size;
    }

    if strings.is_empty() && elements.is_empty() {
        return Err(CoreError::Parse("无法解析 Android binary XML".into()));
    }
    Ok(AxmlReport { strings, elements })
}

fn text_manifest_report(xml: &str) -> AxmlReport {
    let mut strings = Vec::new();
    let mut elements = Vec::new();
    let tag_re = Regex::new(r#"<([A-Za-z0-9_.:-]+)\b([^>]*)>"#).unwrap();
    let attr_re = Regex::new(r#"([A-Za-z0-9_.:-]+)\s*=\s*"([^"]*)""#).unwrap();
    for cap in tag_re.captures_iter(xml) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if raw.starts_with('?') || raw.starts_with('!') || raw.starts_with('/') {
            continue;
        }
        let name = local_xml_name(raw).to_string();
        let attrs_text = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let attrs: Vec<(String, String)> = attr_re
            .captures_iter(attrs_text)
            .map(|a| {
                let key = local_xml_name(a.get(1).map(|m| m.as_str()).unwrap_or("")).to_string();
                let value = xml_unescape(a.get(2).map(|m| m.as_str()).unwrap_or(""));
                strings.push(value.clone());
                (key, value)
            })
            .collect();
        strings.push(name.clone());
        elements.push(AxmlElement { name, attrs });
    }
    AxmlReport { strings, elements }
}

fn attr_value<'a>(el: &'a AxmlElement, name: &str) -> Option<&'a str> {
    el.attrs
        .iter()
        .find(|(k, _)| k == name || k.ends_with(&format!(":{name}")))
        .map(|(_, v)| v.as_str())
}

fn read_apk_manifest(data: Vec<u8>) -> Result<Vec<u8>, CoreError> {
    if let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(data.clone())) {
        if let Some(bytes) = read_zip_entry(&mut zip, "AndroidManifest.xml")? {
            return Ok(bytes);
        }
        return Err(CoreError::Parse("APK 中未找到 AndroidManifest.xml".into()));
    }
    Ok(data)
}

struct ApkManifest;

impl Node for ApkManifest {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "apk")?;
        let manifest = read_apk_manifest(data)?;
        let report = if manifest.iter().take(16).any(|&b| b == b'<') {
            text_manifest_report(&String::from_utf8_lossy(&manifest))
        } else {
            parse_axml(&manifest)?
        };

        let package = report
            .elements
            .iter()
            .find(|e| e.name == "manifest")
            .and_then(|e| attr_value(e, "package"))
            .map(str::to_string);
        let permissions: Vec<String> = report
            .elements
            .iter()
            .filter(|e| e.name == "uses-permission" || e.name == "uses-permission-sdk-23")
            .filter_map(|e| attr_value(e, "name").map(str::to_string))
            .collect();
        let components: Vec<String> = report
            .elements
            .iter()
            .filter(|e| {
                matches!(
                    e.name.as_str(),
                    "activity" | "service" | "receiver" | "provider"
                )
            })
            .filter_map(|e| attr_value(e, "name").map(|v| format!("{}:{v}", e.name)))
            .collect();

        let mut rows = Vec::new();
        rows.push(format!("strings: {}", report.strings.len()));
        rows.push(format!("elements: {}", report.elements.len()));
        if let Some(pkg) = &package {
            rows.push(format!("package: {pkg}"));
        }
        if !permissions.is_empty() {
            rows.push(format!("permissions: {}", permissions.join(", ")));
        }
        if !components.is_empty() {
            rows.push(format!("components: {}", components.join(", ")));
        }

        let elements_json: Vec<_> = report
            .elements
            .iter()
            .map(|e| {
                json!({
                    "name": e.name,
                    "attrs": e.attrs.iter().map(|(k, v)| json!({ "name": k, "value": v })).collect::<Vec<_>>()
                })
            })
            .collect();

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert(
            "json".into(),
            PortValue::Json(json!({
                "package": package,
                "permissions": permissions,
                "components": components,
                "strings": report.strings,
                "elements": elements_json,
            })),
        );
        m.insert("strings".into(), PortValue::StringList(report.strings));
        m.insert(
            "count".into(),
            PortValue::Number(report.elements.len() as f64),
        );
        Ok(m)
    }
}

// ---------------------------------------------------------------- DEX strings

fn read_uleb128(data: &[u8], mut pos: usize) -> Option<(u32, usize)> {
    let start = pos;
    let mut value = 0u32;
    let mut shift = 0u32;
    loop {
        let b = *data.get(pos)?;
        pos += 1;
        value |= ((b & 0x7f) as u32) << shift;
        if b & 0x80 == 0 {
            return Some((value, pos - start));
        }
        shift += 7;
        if shift > 28 {
            return None;
        }
    }
}

struct DexStrings;

impl Node for DexStrings {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "dex")?;
        if data.len() < 0x70 || !data.starts_with(b"dex\n") {
            return Err(CoreError::Parse("不是 DEX 文件（缺少 dex\\n 魔数）".into()));
        }
        let string_ids_size = le32(&data, 0x38).unwrap_or(0) as usize;
        let string_ids_off = le32(&data, 0x3c).unwrap_or(0) as usize;
        let min_len = pnum(p, "minLen", 1.0).max(0.0) as usize;
        let max_count = pnum(p, "maxCount", 2000.0).max(1.0) as usize;
        let show_offset = pbool(p, "showOffset", true);
        if string_ids_off + string_ids_size.saturating_mul(4) > data.len() {
            return Err(CoreError::Parse("DEX string_ids 表超出文件边界".into()));
        }

        let mut strings = Vec::new();
        let mut rows = Vec::new();
        let mut arr = Vec::new();
        for idx in 0..string_ids_size {
            let off = le32(&data, string_ids_off + idx * 4).unwrap_or(0) as usize;
            if off >= data.len() {
                continue;
            }
            let Some((utf16_size, len_len)) = read_uleb128(&data, off) else {
                continue;
            };
            let start = off + len_len;
            let end = data[start..]
                .iter()
                .position(|&b| b == 0)
                .map(|rel| start + rel)
                .unwrap_or(data.len());
            let s = String::from_utf8_lossy(&data[start..end]).to_string();
            if s.chars().count() < min_len {
                continue;
            }
            if strings.len() < max_count {
                if show_offset {
                    rows.push(format!("0x{off:08x}  {s}"));
                } else {
                    rows.push(s.clone());
                }
                strings.push(s.clone());
                arr.push(json!({
                    "index": idx,
                    "offset": off,
                    "utf16Size": utf16_size,
                    "value": s,
                }));
            }
        }

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(rows.join("\n")));
        m.insert("json".into(), PortValue::Json(json!(arr)));
        m.insert("strings".into(), PortValue::StringList(strings.clone()));
        m.insert("count".into(), PortValue::Number(strings.len() as f64));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "pdf_objects",
            UTIL,
            "PDF Object 列表",
            CYAN,
            vec![req("data", "PDF 字节", PortType::Any)],
            vec![
                req("text", "对象列表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("objects", "对象", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(PdfObjects)),
    );
    reg.register(
        desc(
            "pdf_streams",
            UTIL,
            "PDF Stream 列表",
            CYAN,
            vec![req("data", "PDF 字节", PortType::Any)],
            vec![
                req("text", "Stream 列表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("streams", "Stream 对象", PortType::StringList),
                opt("count", "数量", PortType::Number),
                opt("bytes", "首个 Stream", PortType::Bytes),
            ],
            vec![ParamSpec::toggle("decodeFlate", "尝试 FlateDecode", true)],
        ),
        Arc::new(|| Arc::new(PdfStreams)),
    );
    reg.register(
        desc(
            "ooxml_metadata",
            UTIL,
            "OOXML 元数据",
            CYAN,
            vec![req("document", "OOXML 文档", PortType::Any)],
            vec![
                req("text", "属性", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("properties", "属性列表", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(OoxmlMetadata)),
    );
    reg.register(
        desc(
            "ooxml_embedded",
            UTIL,
            "OOXML 内嵌资源",
            CYAN,
            vec![req("document", "OOXML 文档", PortType::Any)],
            vec![
                req("text", "资源列表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("files", "文件", PortType::StringList),
                opt("count", "数量", PortType::Number),
                opt("hasMacro", "含宏", PortType::Bool),
                opt("bytes", "首个资源", PortType::Bytes),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(OoxmlEmbedded)),
    );
    reg.register(
        desc(
            "apk_manifest",
            UTIL,
            "APK Manifest 解析",
            CYAN,
            vec![req("apk", "APK/Manifest", PortType::Any)],
            vec![
                req("text", "摘要", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("strings", "字符串池", PortType::StringList),
                opt("count", "元素数", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(ApkManifest)),
    );
    reg.register(
        desc(
            "dex_strings",
            BIN,
            "DEX 字符串",
            INDIGO,
            vec![req("dex", "DEX 字节", PortType::Any)],
            vec![
                req("text", "字符串", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("strings", "字符串列表", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![
                ParamSpec::number("minLen", "最小长度", 0.0, 256.0, 1.0, 1.0),
                ParamSpec::number("maxCount", "最大输出", 1.0, 100000.0, 1.0, 2000.0),
                ParamSpec::toggle("showOffset", "显示偏移", true),
            ],
        ),
        Arc::new(|| Arc::new(DexStrings)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn run(id: &str, port: &str, data: Vec<u8>, params: serde_json::Value) -> PortMap {
        let mut i = PortMap::new();
        i.insert(
            port.into(),
            PortValue::Bytes(Arc::from(data.into_boxed_slice())),
        );
        GraphExecutor::run_node(
            &default_registry(),
            id,
            &i,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    fn make_pdf() -> Vec<u8> {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(b"hello stream").unwrap();
        let compressed = enc.finish().unwrap();
        let mut pdf =
            b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n2 0 obj\n<< /Length ".to_vec();
        pdf.extend_from_slice(compressed.len().to_string().as_bytes());
        pdf.extend_from_slice(b" /Filter /FlateDecode >>\nstream\n");
        pdf.extend_from_slice(&compressed);
        pdf.extend_from_slice(b"\nendstream\nendobj\n%%EOF\n");
        pdf
    }

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            for (name, data) in entries {
                w.start_file(*name, SimpleFileOptions::default()).unwrap();
                w.write_all(data).unwrap();
            }
            w.finish().unwrap();
        }
        buf
    }

    fn push_u16(v: &mut Vec<u8>, n: u16) {
        v.extend_from_slice(&n.to_le_bytes());
    }

    fn push_u32(v: &mut Vec<u8>, n: u32) {
        v.extend_from_slice(&n.to_le_bytes());
    }

    fn make_string_pool(strings: &[&str]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut offsets = Vec::new();
        for s in strings {
            offsets.push(data.len() as u32);
            data.push(s.len() as u8);
            data.push(s.len() as u8);
            data.extend_from_slice(s.as_bytes());
            data.push(0);
        }
        while data.len() % 4 != 0 {
            data.push(0);
        }

        let header_size = 28u32;
        let strings_start = header_size + strings.len() as u32 * 4;
        let chunk_size = strings_start + data.len() as u32;
        let mut out = Vec::new();
        push_u16(&mut out, 0x0001);
        push_u16(&mut out, header_size as u16);
        push_u32(&mut out, chunk_size);
        push_u32(&mut out, strings.len() as u32);
        push_u32(&mut out, 0);
        push_u32(&mut out, 0x0000_0100);
        push_u32(&mut out, strings_start);
        push_u32(&mut out, 0);
        for off in offsets {
            push_u32(&mut out, off);
        }
        out.extend_from_slice(&data);
        out
    }

    fn make_axml_manifest() -> Vec<u8> {
        let pool = make_string_pool(&["manifest", "package", "com.example.app"]);
        let mut start = Vec::new();
        push_u16(&mut start, 0x0102);
        push_u16(&mut start, 16);
        push_u32(&mut start, 56);
        push_u32(&mut start, 1);
        push_u32(&mut start, u32::MAX);
        push_u32(&mut start, u32::MAX);
        push_u32(&mut start, 0);
        push_u16(&mut start, 20);
        push_u16(&mut start, 20);
        push_u16(&mut start, 1);
        push_u16(&mut start, 0);
        push_u16(&mut start, 0);
        push_u16(&mut start, 0);
        push_u32(&mut start, u32::MAX);
        push_u32(&mut start, 1);
        push_u32(&mut start, 2);
        push_u16(&mut start, 8);
        start.push(0);
        start.push(0x03);
        push_u32(&mut start, 2);

        let total = 8 + pool.len() + start.len();
        let mut axml = Vec::new();
        push_u16(&mut axml, 0x0003);
        push_u16(&mut axml, 8);
        push_u32(&mut axml, total as u32);
        axml.extend_from_slice(&pool);
        axml.extend_from_slice(&start);
        axml
    }

    fn make_dex() -> Vec<u8> {
        let mut dex = vec![0u8; 0x74];
        dex[0..8].copy_from_slice(b"dex\n035\0");
        dex[0x38..0x3c].copy_from_slice(&1u32.to_le_bytes());
        dex[0x3c..0x40].copy_from_slice(&0x70u32.to_le_bytes());
        dex[0x70..0x74].copy_from_slice(&0x74u32.to_le_bytes());
        dex.push(5);
        dex.extend_from_slice(b"hello");
        dex.push(0);
        dex
    }

    #[test]
    fn pdf_objects_and_streams_report() {
        let pdf = make_pdf();
        let out = run("pdf_objects", "data", pdf.clone(), json!({}));
        assert!(matches!(out.get("count"), Some(PortValue::Number(n)) if *n == 2.0));
        assert!(matches!(out.get("text"), Some(PortValue::Text(t)) if t.contains("Catalog")));

        let out = run("pdf_streams", "data", pdf, json!({ "decodeFlate": true }));
        assert!(matches!(out.get("text"), Some(PortValue::Text(t)) if t.contains("hello stream")));
        assert!(
            matches!(out.get("bytes"), Some(PortValue::Bytes(b)) if b.as_ref() == b"hello stream")
        );
    }

    #[test]
    fn ooxml_metadata_and_embedded_report() {
        let doc = make_zip(&[
            (
                "docProps/core.xml",
                br#"<cp:coreProperties><dc:title>Test title</dc:title><dc:creator>Alice</dc:creator></cp:coreProperties>"#,
            ),
            ("word/embeddings/oleObject1.bin", b"OLE"),
        ]);
        let meta = run("ooxml_metadata", "document", doc.clone(), json!({}));
        assert!(matches!(meta.get("text"), Some(PortValue::Text(t)) if t.contains("Test title")));

        let embedded = run("ooxml_embedded", "document", doc, json!({}));
        assert!(matches!(embedded.get("count"), Some(PortValue::Number(n)) if *n == 1.0));
        assert!(matches!(embedded.get("bytes"), Some(PortValue::Bytes(b)) if b.as_ref() == b"OLE"));
    }

    #[test]
    fn apk_manifest_reads_binary_xml() {
        let apk = make_zip(&[("AndroidManifest.xml", &make_axml_manifest())]);
        let out = run("apk_manifest", "apk", apk, json!({}));
        assert!(
            matches!(out.get("text"), Some(PortValue::Text(t)) if t.contains("com.example.app"))
        );
        assert!(
            matches!(out.get("strings"), Some(PortValue::StringList(v)) if v.iter().any(|s| s == "manifest"))
        );
    }

    #[test]
    fn dex_strings_reads_string_ids() {
        let out = run("dex_strings", "dex", make_dex(), json!({}));
        assert!(
            matches!(out.get("strings"), Some(PortValue::StringList(v)) if v == &vec!["hello".to_string()])
        );
    }
}
