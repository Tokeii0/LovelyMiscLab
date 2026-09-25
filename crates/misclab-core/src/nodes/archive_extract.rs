use std::io::{Cursor, Read};
use std::path::Path;

use base64::Engine as _;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use serde_json::json;

use super::prelude::*;

fn decode_data_url_text(s: &str) -> Option<Result<Vec<u8>, CoreError>> {
    let rest = s.trim().strip_prefix("data:")?;
    let comma = match rest.find(',') {
        Some(comma) => comma,
        None => {
            return Some(Err(CoreError::Parse(
                "压缩包 data URL 缺少逗号分隔符".into(),
            )))
        }
    };
    let (meta, payload) = (&rest[..comma], rest[comma + 1..].trim());
    if meta.contains(";base64") {
        Some(
            base64::engine::general_purpose::STANDARD
                .decode(payload)
                .map_err(|e| CoreError::Parse(format!("压缩包 data URL base64 解码失败: {e}"))),
        )
    } else {
        Some(Ok(payload.as_bytes().to_vec()))
    }
}

fn looks_like_path(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && !s.contains('\n')
        && !s.contains('\r')
        && (s.contains('\\')
            || s.contains('/')
            || s.get(1..3).is_some_and(|drive| {
                let bytes = drive.as_bytes();
                bytes.len() == 2 && bytes[0] == b':' && (bytes[1] == b'\\' || bytes[1] == b'/')
            }))
}

fn compact_base64_candidate(s: &str) -> Option<String> {
    let compact = s.split_whitespace().collect::<String>();
    if compact.len() < 16 || compact.len() % 4 == 1 {
        return None;
    }
    let valid = compact
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '-' | '_' | '='));
    valid.then_some(compact)
}

fn decode_base64_archive_text(s: &str) -> Option<Vec<u8>> {
    let compact = compact_base64_candidate(s)?;
    let mut padded = compact.trim_end_matches('=').to_string();
    let pad = (4 - padded.len() % 4) % 4;
    padded.extend(std::iter::repeat_n('=', pad));
    for engine in [
        &base64::engine::general_purpose::STANDARD,
        &base64::engine::general_purpose::URL_SAFE,
    ] {
        if let Ok(bytes) = engine.decode(&padded) {
            if detect(&bytes) != "unknown" {
                return Some(bytes);
            }
        }
    }
    None
}

fn archive_input_bytes(inputs: &PortMap, name: &str) -> Result<Vec<u8>, CoreError> {
    match inputs.get(name) {
        Some(PortValue::Text(s)) => {
            if let Some(decoded) = decode_data_url_text(s) {
                return decoded;
            }
            if looks_like_path(s) {
                let path = s.trim().trim_matches('"');
                if std::path::Path::new(path).is_file() {
                    return std::fs::read(path)
                        .map_err(|e| CoreError::Other(format!("读取压缩包文件失败: {e}")));
                }
            }
            if let Some(bytes) = decode_base64_archive_text(s) {
                return Ok(bytes);
            }
            Ok(s.as_bytes().to_vec())
        }
        _ => in_bytes(inputs, name),
    }
}

/// Largest single entry we decompress (zip-bomb guard).
const MAX_ENTRY: u64 = 64 * 1024 * 1024;
/// Total decompressed bytes per archive.
const MAX_TOTAL: u64 = 256 * 1024 * 1024;
/// How much entry data is embedded (as base64) in the `entries` JSON output.
const MAX_EMBED: usize = 32 * 1024 * 1024;
/// `EntryData::error` for an encrypted entry read without a password.
const NEED_PASSWORD: &str = "需要密码";

/// Read at most `min(MAX_ENTRY, budget)` bytes; more than that is an error
/// (reported on the entry) instead of an unbounded allocation.
fn read_capped(r: &mut dyn Read, budget: &mut u64) -> Result<Vec<u8>, String> {
    let limit = MAX_ENTRY.min(*budget);
    let mut buf = Vec::new();
    r.take(limit + 1)
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    if buf.len() as u64 > limit {
        return Err(if limit < MAX_ENTRY {
            "解压总量超过 256MB 上限，未读取".into()
        } else {
            "条目超过 64MB 上限，未读取".into()
        });
    }
    *budget -= buf.len() as u64;
    Ok(buf)
}

fn too_big(size: u64) -> Option<String> {
    (size > MAX_ENTRY).then(|| format!("条目过大（{size} 字节，上限 64MB），未读取"))
}

/// Map a library error that is really "wrong / missing password".
fn password_error(msg: &str, password: &str) -> Option<CoreError> {
    msg.to_ascii_lowercase().contains("password").then(|| {
        CoreError::PasswordRequired(if password.is_empty() {
            "压缩包已加密，需要密码".into()
        } else {
            "密码错误，无法解密".into()
        })
    })
}

#[derive(Debug, Clone)]
struct EntryData {
    name: String,
    is_dir: bool,
    size: u64,
    compressed_size: Option<u64>,
    encrypted: bool,
    crc32: Option<u32>,
    bytes: Option<Vec<u8>>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct ArchiveData {
    format: String,
    entries: Vec<EntryData>,
}

fn detect(data: &[u8]) -> &'static str {
    if data.starts_with(b"PK\x03\x04")
        || data.starts_with(b"PK\x05\x06")
        || data.starts_with(b"PK\x07\x08")
    {
        "zip"
    } else if data.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        "7z"
    } else if data.starts_with(&[0x1F, 0x8B]) {
        "gz"
    } else if data.starts_with(b"Rar!") {
        "rar"
    } else if data.len() > 262 && &data[257..262] == b"ustar" {
        "tar"
    } else if looks_like_zlib(data) {
        "zlib"
    } else {
        "unknown"
    }
}

fn looks_like_zlib(data: &[u8]) -> bool {
    if data.len() < 2 {
        return false;
    }
    let cmf = data[0];
    let flg = data[1];
    cmf & 0x0f == 8 && ((u16::from(cmf) << 8) | u16::from(flg)) % 31 == 0
}

fn normalize_format(fmt: &str) -> String {
    match fmt.trim().to_ascii_lowercase().as_str() {
        "" | "自动" | "auto" => "auto".into(),
        "zip" => "zip".into(),
        "7z" | "7zip" => "7z".into(),
        "rar" => "rar".into(),
        "gz" | "gzip" => "gz".into(),
        "tar.gz" | "tgz" => "tar.gz".into(),
        "tar" => "tar".into(),
        "zlib" => "zlib".into(),
        "deflate" | "raw deflate" | "raw_deflate" => "deflate".into(),
        other => other.into(),
    }
}

fn single_file(format: &str, name: &str, bytes: Vec<u8>) -> ArchiveData {
    ArchiveData {
        format: format.into(),
        entries: vec![EntryData {
            name: name.into(),
            is_dir: false,
            size: bytes.len() as u64,
            compressed_size: None,
            encrypted: false,
            crc32: None,
            bytes: Some(bytes),
            error: None,
        }],
    }
}

fn extract_zip(data: &[u8], password: &str) -> Result<ArchiveData, CoreError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(data))
        .map_err(|e| CoreError::Parse(format!("zip: {e}")))?;
    let mut entries = Vec::with_capacity(zip.len());
    let mut budget = MAX_TOTAL;

    for i in 0..zip.len() {
        let mut entry = {
            let f = zip
                .by_index_raw(i)
                .map_err(|e| CoreError::Parse(format!("读取 ZIP 条目 {i} 失败: {e}")))?;
            EntryData {
                name: f.name().to_string(),
                is_dir: f.is_dir(),
                size: f.size(),
                compressed_size: Some(f.compressed_size()),
                encrypted: f.encrypted(),
                crc32: Some(f.crc32()),
                bytes: None,
                error: None,
            }
        };

        if !entry.is_dir {
            if entry.encrypted && password.is_empty() {
                entry.error = Some(NEED_PASSWORD.into());
            } else if let Some(msg) = too_big(entry.size) {
                entry.error = Some(msg);
            } else {
                let read_result = if entry.encrypted {
                    let mut f =
                        zip.by_index_decrypt(i, password.as_bytes())
                            .map_err(|e| match e {
                                zip::result::ZipError::InvalidPassword => {
                                    CoreError::PasswordRequired("密码错误，无法解密".into())
                                }
                                other => CoreError::Parse(format!("ZIP 解密失败: {other}")),
                            })?;
                    read_capped(&mut f, &mut budget)
                } else {
                    let mut f = zip
                        .by_index(i)
                        .map_err(|e| CoreError::Parse(format!("读取 ZIP 条目 {i} 失败: {e}")))?;
                    read_capped(&mut f, &mut budget)
                };
                match read_result {
                    Ok(buf) => entry.bytes = Some(buf),
                    Err(e) => entry.error = Some(e),
                }
            }
        }
        entries.push(entry);
    }

    Ok(ArchiveData {
        format: "zip".into(),
        entries,
    })
}

fn gz_filename(data: &[u8]) -> String {
    let decoder = GzDecoder::new(Cursor::new(data));
    decoder
        .header()
        .and_then(|h| h.filename())
        .filter(|name| !name.is_empty())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .unwrap_or_else(|| "(gzip 解压内容)".into())
}

fn decode_gz(data: &[u8]) -> Result<Vec<u8>, CoreError> {
    read_capped(&mut GzDecoder::new(Cursor::new(data)), &mut { MAX_TOTAL })
        .map_err(|e| CoreError::Parse(format!("gzip: {e}")))
}

fn extract_gz(data: &[u8], auto_nested_tar: bool) -> Result<ArchiveData, CoreError> {
    let name = gz_filename(data);
    let bytes = decode_gz(data)?;
    if auto_nested_tar && detect(&bytes) == "tar" {
        let mut out = extract_tar(&bytes)?;
        out.format = "tar.gz".into();
        return Ok(out);
    }
    Ok(single_file("gz", &name, bytes))
}

fn extract_zlib(data: &[u8]) -> Result<ArchiveData, CoreError> {
    let buf = read_capped(&mut ZlibDecoder::new(Cursor::new(data)), &mut { MAX_TOTAL })
        .map_err(|e| CoreError::Parse(format!("zlib: {e}")))?;
    Ok(single_file("zlib", "(zlib 解压内容)", buf))
}

fn extract_deflate(data: &[u8]) -> Result<ArchiveData, CoreError> {
    let buf = read_capped(&mut DeflateDecoder::new(Cursor::new(data)), &mut {
        MAX_TOTAL
    })
    .map_err(|e| CoreError::Parse(format!("raw deflate: {e}")))?;
    Ok(single_file("deflate", "(raw deflate 解压内容)", buf))
}

fn extract_tar(data: &[u8]) -> Result<ArchiveData, CoreError> {
    let mut archive = tar::Archive::new(Cursor::new(data));
    let mut entries = Vec::new();
    let mut budget = MAX_TOTAL;

    for entry in archive
        .entries()
        .map_err(|e| CoreError::Parse(format!("tar: {e}")))?
    {
        let mut e = entry.map_err(|e| CoreError::Parse(format!("tar: {e}")))?;
        let name = e
            .path()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let is_file = e.header().entry_type().is_file();
        let is_dir = e.header().entry_type().is_dir();
        let size = e.header().size().unwrap_or(0);
        let mut item = EntryData {
            name,
            is_dir,
            size,
            compressed_size: None,
            encrypted: false,
            crc32: None,
            bytes: None,
            error: None,
        };
        if is_file {
            if let Some(msg) = too_big(size) {
                item.error = Some(msg);
            } else {
                match read_capped(&mut e, &mut budget) {
                    Ok(buf) => {
                        item.size = buf.len() as u64;
                        item.bytes = Some(buf);
                    }
                    Err(msg) => item.error = Some(msg),
                }
            }
        }
        entries.push(item);
    }

    Ok(ArchiveData {
        format: "tar".into(),
        entries,
    })
}

fn extract_7z(data: &[u8], password: &str) -> Result<ArchiveData, CoreError> {
    let map_err = |e: sevenz_rust::Error| {
        let msg = e.to_string();
        password_error(&msg, password).unwrap_or_else(|| CoreError::Parse(format!("7z: {msg}")))
    };
    let mut reader = sevenz_rust::SevenZReader::new(
        Cursor::new(data),
        data.len() as u64,
        sevenz_rust::Password::from(password),
    )
    .map_err(map_err)?;

    let mut entries = Vec::new();
    let mut budget = MAX_TOTAL;
    reader
        .for_each_entries(|entry, rd| {
            let mut item = EntryData {
                name: entry.name().replace('\\', "/"),
                is_dir: entry.is_directory(),
                size: entry.size(),
                compressed_size: Some(entry.compressed_size),
                encrypted: false,
                crc32: entry.has_crc.then_some(entry.crc as u32),
                bytes: None,
                error: None,
            };
            if !entry.is_directory() {
                match read_capped(rd, &mut budget) {
                    Ok(buf) => {
                        item.size = buf.len() as u64;
                        item.bytes = Some(buf);
                    }
                    Err(msg) => {
                        // Drain the rest so the next entry of a solid archive decodes.
                        std::io::copy(rd, &mut std::io::sink())?;
                        item.error = Some(msg);
                    }
                }
            }
            entries.push(item);
            Ok(true)
        })
        .map_err(map_err)?;

    Ok(ArchiveData {
        format: "7z".into(),
        entries,
    })
}

fn extract_rar(data: &[u8], password: &str) -> Result<ArchiveData, CoreError> {
    let tmp = std::env::temp_dir().join(format!("misclab_{}.rar", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, data).map_err(|e| CoreError::Other(e.to_string()))?;
    let result = rar_inner(&tmp, password);
    std::fs::remove_file(&tmp).ok();
    result
}

fn rar_inner(path: &Path, password: &str) -> Result<ArchiveData, CoreError> {
    let mut archive = if password.is_empty() {
        unrar::Archive::new(path).open_for_processing()
    } else {
        unrar::Archive::with_password(path, password).open_for_processing()
    }
    .map_err(|e| {
        password_error(&e.to_string(), password)
            .unwrap_or_else(|| CoreError::Parse(format!("rar: {e}")))
    })?;

    let mut entries = Vec::new();
    let mut total = 0u64;
    while let Some(header) = archive.read_header().map_err(|e| {
        password_error(&e.to_string(), password)
            .unwrap_or_else(|| CoreError::Parse(format!("rar: {e}")))
    })? {
        let meta = header.entry();
        let name = meta.filename.to_string_lossy().replace('\\', "/");
        let is_dir = meta.is_directory();
        let encrypted = meta.is_encrypted();
        let size = meta.unpacked_size;
        let crc32 = Some(meta.file_crc);

        let mut item = EntryData {
            name,
            is_dir,
            size,
            compressed_size: None,
            encrypted,
            crc32,
            bytes: None,
            error: None,
        };

        let over = too_big(size).or_else(|| {
            (total + size > MAX_TOTAL).then(|| "解压总量超过 256MB 上限，未读取".to_string())
        });
        if meta.is_file() && over.is_none() {
            let (bytes, rest) = header.read().map_err(|e| {
                password_error(&e.to_string(), password)
                    .unwrap_or_else(|| CoreError::Parse(format!("rar 读取失败: {e}")))
            })?;
            total += bytes.len() as u64;
            item.size = bytes.len() as u64;
            item.bytes = Some(bytes);
            archive = rest;
        } else {
            item.error = over;
            archive = header
                .skip()
                .map_err(|e| CoreError::Parse(format!("rar 跳过目录失败: {e}")))?;
        }
        entries.push(item);
    }

    Ok(ArchiveData {
        format: "rar".into(),
        entries,
    })
}

fn extract_archive(data: &[u8], fmt: &str, password: &str) -> Result<ArchiveData, CoreError> {
    let requested = normalize_format(fmt);
    let auto = requested == "auto";
    let kind = if auto {
        detect(data).to_string()
    } else {
        requested
    };

    match kind.as_str() {
        "zip" => extract_zip(data, password),
        "gz" => extract_gz(data, auto),
        "tar.gz" => {
            let bytes = decode_gz(data)?;
            let mut out = extract_tar(&bytes)?;
            out.format = "tar.gz".into();
            Ok(out)
        }
        "zlib" => extract_zlib(data),
        "deflate" => extract_deflate(data),
        "tar" => extract_tar(data),
        "7z" => extract_7z(data, password),
        "rar" => extract_rar(data, password),
        "unknown" => Err(CoreError::Unsupported(
            "未识别的压缩格式；如果输入来自故事模式或文件节点，请确认传入的是文件字节、data URL、文件路径或压缩包 base64，而不是文件名/列表文本。可手动选择 zip/7z/rar/gz/tar/zlib/deflate".into(),
        )),
        other => Err(CoreError::Unsupported(format!("不支持的压缩格式: {other}"))),
    }
}

fn readable_files(entries: &[EntryData]) -> Vec<&EntryData> {
    entries
        .iter()
        .filter(|entry| !entry.is_dir && entry.bytes.is_some())
        .collect()
}

fn file_names(entries: &[EntryData]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .map(|entry| entry.name.clone())
        .collect()
}

fn find_selected<'a>(
    entries: &'a [EntryData],
    target: &str,
) -> Result<Option<&'a EntryData>, CoreError> {
    let target = target.trim();
    if target.is_empty() {
        return Ok(readable_files(entries).into_iter().next());
    }

    if let Some(entry) = entries.iter().find(|entry| entry.name == target) {
        return if entry.is_dir {
            Err(CoreError::Parse(format!("指定条目是目录: {target}")))
        } else {
            Ok(Some(entry))
        };
    }

    if let Ok(index) = target.parse::<usize>() {
        if let Some(entry) = entries.iter().filter(|entry| !entry.is_dir).nth(index) {
            return Ok(Some(entry));
        }
    }

    let preview = file_names(entries).into_iter().take(8).collect::<Vec<_>>();
    Err(CoreError::Parse(format!(
        "找不到压缩包条目: {target}。可选文件: {}",
        preview.join(", ")
    )))
}

fn entries_json(data: &ArchiveData) -> serde_json::Value {
    let mut file_index = 0usize;
    let mut embedded = 0usize;
    let entries = data
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let current_file_index = if entry.is_dir {
                None
            } else {
                let n = file_index;
                file_index += 1;
                Some(n)
            };
            json!({
                "index": index,
                "fileIndex": current_file_index,
                "name": &entry.name,
                "isDir": entry.is_dir,
                "size": entry.size,
                "compressedSize": entry.compressed_size,
                "encrypted": entry.encrypted,
                "crc32": entry.crc32.map(|crc| format!("{crc:08x}")),
                "error": entry.error,
                "dataBase64": entry.bytes.as_ref().and_then(|bytes| {
                    embedded += bytes.len();
                    (embedded <= MAX_EMBED)
                        .then(|| base64::engine::general_purpose::STANDARD.encode(bytes))
                }),
                "dataOmitted": entry.bytes.is_some() && embedded > MAX_EMBED,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "kind": "misclab.archive.entries",
        "version": 1,
        "format": &data.format,
        "entries": entries,
    })
}

fn finish(data: ArchiveData, target: &str) -> Result<PortMap, CoreError> {
    // Nothing readable without a password (or the requested entry needs one): an
    // error, not an empty "success" that silently feeds nothing downstream.
    let locked = data
        .entries
        .iter()
        .filter(|e| e.error.as_deref() == Some(NEED_PASSWORD))
        .count();
    if locked > 0 && readable_files(&data.entries).is_empty() {
        return Err(CoreError::PasswordRequired(format!(
            "压缩包已加密，需要密码（{locked} 个加密条目）"
        )));
    }
    let selected = find_selected(&data.entries, target)?;
    if let Some(entry) = selected {
        if entry.error.as_deref() == Some(NEED_PASSWORD) {
            return Err(CoreError::PasswordRequired(format!(
                "条目 {} 已加密，需要密码",
                entry.name
            )));
        }
    }
    if !target.trim().is_empty() {
        if let Some(entry) = selected {
            if let Some(error) = &entry.error {
                return Err(CoreError::Parse(format!(
                    "条目 {} 无法读取: {error}",
                    entry.name
                )));
            }
            if entry.bytes.is_none() {
                return Err(CoreError::Parse(format!(
                    "条目 {} 没有可读取内容",
                    entry.name
                )));
            }
        }
    }

    let files = file_names(&data.entries);
    let dir_count = data.entries.iter().filter(|entry| entry.is_dir).count();
    let error_count = data
        .entries
        .iter()
        .filter(|entry| entry.error.is_some())
        .count();
    let selected_name = selected.map(|entry| entry.name.clone()).unwrap_or_default();
    let bytes = selected
        .and_then(|entry| entry.bytes.clone())
        .unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let mut summary = format!("{}: {} 个文件", data.format, files.len());
    if dir_count > 0 {
        summary.push_str(&format!("，{dir_count} 个目录"));
    }
    if selected_name.is_empty() {
        summary.push_str("；未找到可直接输出的文件");
    } else {
        summary.push_str(&format!(
            "；当前输出「{selected_name}」（{} 字节）",
            bytes.len()
        ));
    }
    if error_count > 0 {
        summary.push_str(&format!("；{error_count} 个条目未读取"));
    }
    summary.push('。');

    let mut out = PortMap::new();
    out.insert("files".into(), PortValue::StringList(files));
    out.insert("entries".into(), PortValue::Json(entries_json(&data)));
    out.insert("text".into(), PortValue::Text(text));
    out.insert(
        "bytes".into(),
        PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
    );
    out.insert("entry".into(), PortValue::Text(selected_name));
    out.insert(
        "count".into(),
        PortValue::Number(data.entries.iter().filter(|entry| !entry.is_dir).count() as f64),
    );
    out.insert("format".into(), PortValue::Text(data.format));
    out.insert("summary".into(), PortValue::Text(summary));
    Ok(out)
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = archive_input_bytes(inputs, "archive")?;
        let password = pstr(params, "password", "");
        let target = pstr(params, "entry", "");
        let fmt = pstr(params, "format", "自动");
        let archive = extract_archive(&data, fmt, password)?;
        finish(archive, target)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "archive_extract",
            ARC,
            "解压",
            "#f97316",
            vec![req("archive", "压缩包", PortType::Any)],
            vec![
                req("files", "文件列表", PortType::StringList),
                opt("entries", "解压结果", PortType::Json),
                opt("text", "当前内容", PortType::Text),
                opt("bytes", "当前字节", PortType::Bytes),
                opt("entry", "当前文件名", PortType::Text),
                opt("count", "文件数", PortType::Number),
                opt("format", "格式", PortType::Text),
                opt("summary", "摘要", PortType::Text),
            ],
            vec![
                ParamSpec::select(
                    "format",
                    "格式",
                    &[
                        "自动", "zip", "7z", "rar", "gz", "tar.gz", "tar", "zlib", "deflate",
                    ],
                    "自动",
                ),
                ParamSpec::text("password", "密码", "", false),
                ParamSpec::text("entry", "指定条目/序号(可选)", "", false),
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
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn run_extract(bytes: Vec<u8>, params: serde_json::Value) -> PortMap {
        run_extract_value(
            PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
            params,
        )
    }

    fn run_extract_value(value: PortValue, params: serde_json::Value) -> PortMap {
        let mut inputs = PortMap::new();
        inputs.insert("archive".into(), value);
        GraphExecutor::run_node(
            &default_registry(),
            "archive_extract",
            &inputs,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    fn text(out: &PortMap, name: &str) -> String {
        match out.get(name) {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("expected text {name}, got {o:?}"),
        }
    }

    fn bytes(out: &PortMap, name: &str) -> Vec<u8> {
        match out.get(name) {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("expected bytes {name}, got {o:?}"),
        }
    }

    fn make_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            w.add_directory("dir/", opts).unwrap();
            w.start_file("a.txt", opts).unwrap();
            w.write_all(b"alpha").unwrap();
            w.start_file("dir/b.bin", opts).unwrap();
            w.write_all(&[0, 1, 2, 3]).unwrap();
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn zip_outputs_file_list_bundle_and_first_file() {
        let out = run_extract(make_zip(), json!({}));
        assert_eq!(text(&out, "entry"), "a.txt");
        assert_eq!(text(&out, "text"), "alpha");
        assert_eq!(bytes(&out, "bytes"), b"alpha");
        assert!(text(&out, "summary").contains("2 个文件"));
        assert!(matches!(out.get("count"), Some(PortValue::Number(n)) if *n == 2.0));
        let files = match out.get("files") {
            Some(PortValue::StringList(v)) => v.clone(),
            o => panic!("{o:?}"),
        };
        assert_eq!(files, vec!["a.txt", "dir/b.bin"]);
        let entries = match out.get("entries") {
            Some(PortValue::Json(v)) => v,
            o => panic!("{o:?}"),
        };
        assert_eq!(entries["format"], "zip");
        assert_eq!(entries["entries"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn zip_can_select_file_by_name_or_index() {
        let by_name = run_extract(make_zip(), json!({ "entry": "dir/b.bin" }));
        assert_eq!(bytes(&by_name, "bytes"), vec![0, 1, 2, 3]);
        let by_index = run_extract(make_zip(), json!({ "entry": "1" }));
        assert_eq!(text(&by_index, "entry"), "dir/b.bin");
        assert_eq!(bytes(&by_index, "bytes"), vec![0, 1, 2, 3]);
    }

    #[test]
    fn zip_accepts_text_data_url_and_bare_base64() {
        let zip = make_zip();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&zip);
        let data_url = format!("data:application/zip;base64,{encoded}");

        let from_data_url =
            run_extract_value(PortValue::Text(data_url), json!({ "format": "zip" }));
        assert_eq!(text(&from_data_url, "entry"), "a.txt");
        assert_eq!(text(&from_data_url, "text"), "alpha");

        let from_base64 = run_extract_value(PortValue::Text(encoded), json!({ "format": "自动" }));
        assert_eq!(text(&from_base64, "format"), "zip");
        assert_eq!(text(&from_base64, "entry"), "a.txt");
    }

    fn make_encrypted_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes256, "pw");
            w.start_file("flag.txt", opts).unwrap();
            w.write_all(b"flag{zip}").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    fn run_err(bytes: Vec<u8>, params: serde_json::Value) -> CoreError {
        let mut inputs = PortMap::new();
        inputs.insert(
            "archive".into(),
            PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
        );
        GraphExecutor::run_node(
            &default_registry(),
            "archive_extract",
            &inputs,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap_err()
    }

    #[test]
    fn encrypted_zip_without_password_is_an_error_not_empty_output() {
        let err = run_err(make_encrypted_zip(), json!({}));
        assert!(matches!(err, CoreError::PasswordRequired(_)), "{err:?}");
        let wrong = run_err(make_encrypted_zip(), json!({ "password": "nope" }));
        assert!(matches!(wrong, CoreError::PasswordRequired(ref m) if m.contains("密码错误")));
        let ok = run_extract(make_encrypted_zip(), json!({ "password": "pw" }));
        assert_eq!(text(&ok, "text"), "flag{zip}");
    }

    #[test]
    fn oversized_entries_are_reported_not_decompressed() {
        let mut budget = 16u64;
        let mut small = Cursor::new(vec![7u8; 10]);
        assert_eq!(read_capped(&mut small, &mut budget).unwrap().len(), 10);
        assert_eq!(budget, 6);
        let mut big = Cursor::new(vec![7u8; 10]);
        assert!(read_capped(&mut big, &mut budget).is_err());
        assert!(too_big(MAX_ENTRY + 1).is_some());
        assert!(too_big(MAX_ENTRY).is_none());
    }

    #[test]
    fn tar_extracts_multiple_files() {
        let mut buf = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut buf);
            let mut h = tar::Header::new_gnu();
            h.set_size(5);
            h.set_mode(0o644);
            h.set_cksum();
            builder
                .append_data(&mut h, "one.txt", Cursor::new(b"first"))
                .unwrap();
            let mut h = tar::Header::new_gnu();
            h.set_size(6);
            h.set_mode(0o644);
            h.set_cksum();
            builder
                .append_data(&mut h, "two.txt", Cursor::new(b"second"))
                .unwrap();
            builder.finish().unwrap();
        }
        let out = run_extract(buf, json!({ "format": "tar", "entry": "two.txt" }));
        assert_eq!(text(&out, "format"), "tar");
        assert_eq!(text(&out, "text"), "second");
    }

    #[test]
    fn deflate_family_roundtrips() {
        for format in ["Gzip", "Zlib", "Raw Deflate"] {
            let mut inputs = PortMap::new();
            inputs.insert("data".into(), PortValue::Text("hello archive".into()));
            let compressed = GraphExecutor::run_node(
                &default_registry(),
                "compress",
                &inputs,
                &json!({ "format": format }),
                &NullSink,
                &CancellationToken::new(),
            )
            .unwrap();
            let data = bytes(&compressed, "bytes");
            let extract_format = match format {
                "Zlib" => "zlib",
                "Raw Deflate" => "deflate",
                _ => "自动",
            };
            let out = run_extract(data, json!({ "format": extract_format }));
            assert_eq!(text(&out, "text"), "hello archive");
        }
    }
}
