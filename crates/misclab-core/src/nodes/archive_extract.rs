use std::io::{Cursor, Read};
use std::path::Path;

use super::prelude::*;

fn finish(names: Vec<String>, content: Option<Vec<u8>>) -> PortMap {
    let bytes = content.unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let mut out = PortMap::new();
    out.insert("files".to_string(), PortValue::StringList(names));
    out.insert("text".to_string(), PortValue::Text(text));
    out.insert(
        "bytes".to_string(),
        PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
    );
    out
}

fn detect(data: &[u8]) -> &'static str {
    if data.starts_with(b"PK\x03\x04") || data.starts_with(b"PK\x05\x06") {
        "zip"
    } else if data.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        "7z"
    } else if data.starts_with(&[0x1F, 0x8B]) {
        "gz"
    } else if data.starts_with(b"Rar!") {
        "rar"
    } else if data.len() > 262 && &data[257..262] == b"ustar" {
        "tar"
    } else {
        "unknown"
    }
}

fn extract_zip(data: &[u8], password: &str, target: &str) -> Result<PortMap, CoreError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(data))
        .map_err(|e| CoreError::Parse(format!("zip: {e}")))?;
    let names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.name_for_index(i).map(|s| s.to_string()))
        .collect();

    let idx = if target.is_empty() {
        (0..zip.len()).find(|&i| zip.by_index(i).map(|f| f.is_file()).unwrap_or(false))
    } else {
        names.iter().position(|n| n == target)
    };

    let content = match idx {
        Some(i) => {
            let mut buf = Vec::new();
            if password.is_empty() {
                zip.by_index(i)
                    .map_err(|e| CoreError::Parse(format!("zip: {e}")))?
                    .read_to_end(&mut buf)?;
            } else {
                zip.by_index_decrypt(i, password.as_bytes())
                    .map_err(|e| CoreError::Parse(format!("zip 解密失败: {e}")))?
                    .read_to_end(&mut buf)?;
            }
            Some(buf)
        }
        None => None,
    };
    Ok(finish(names, content))
}

fn extract_gz(data: &[u8]) -> Result<PortMap, CoreError> {
    let mut buf = Vec::new();
    flate2::read::GzDecoder::new(Cursor::new(data))
        .read_to_end(&mut buf)
        .map_err(|e| CoreError::Parse(format!("gz: {e}")))?;
    Ok(finish(vec!["(gzip 单文件)".to_string()], Some(buf)))
}

fn extract_tar(data: &[u8], target: &str) -> Result<PortMap, CoreError> {
    let mut archive = tar::Archive::new(Cursor::new(data));
    let mut names = Vec::new();
    let mut content = None;
    for entry in archive
        .entries()
        .map_err(|e| CoreError::Parse(format!("tar: {e}")))?
    {
        let mut e = entry.map_err(|e| CoreError::Parse(format!("tar: {e}")))?;
        let name = e
            .path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let is_file = e.header().entry_type().is_file();
        names.push(name.clone());
        if is_file && content.is_none() && (target.is_empty() || name == target) {
            let mut buf = Vec::new();
            e.read_to_end(&mut buf)?;
            content = Some(buf);
        }
    }
    Ok(finish(names, content))
}

fn extract_7z(data: &[u8], password: &str, target: &str) -> Result<PortMap, CoreError> {
    let mut reader = sevenz_rust::SevenZReader::new(
        Cursor::new(data),
        data.len() as u64,
        sevenz_rust::Password::from(password),
    )
    .map_err(|e| CoreError::Parse(format!("7z: {e}")))?;

    let mut names = Vec::new();
    let mut content: Option<Vec<u8>> = None;
    reader
        .for_each_entries(|entry, rd| {
            let name = entry.name().to_string();
            names.push(name.clone());
            if !entry.is_directory() && content.is_none() && (target.is_empty() || name == target) {
                let mut buf = Vec::new();
                if std::io::copy(rd, &mut buf).is_ok() {
                    content = Some(buf);
                }
            }
            Ok(true)
        })
        .map_err(|e| CoreError::Parse(format!("7z: {e}")))?;
    Ok(finish(names, content))
}

fn extract_rar(data: &[u8], password: &str, target: &str) -> Result<PortMap, CoreError> {
    let tmp = std::env::temp_dir().join(format!("misclab_{}.rar", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, data).map_err(|e| CoreError::Other(e.to_string()))?;
    let result = rar_inner(&tmp, password, target);
    std::fs::remove_file(&tmp).ok();
    result
}

fn rar_inner(path: &Path, password: &str, target: &str) -> Result<PortMap, CoreError> {
    let open_listing = if password.is_empty() {
        unrar::Archive::new(path).open_for_listing()
    } else {
        unrar::Archive::with_password(path, password).open_for_listing()
    };
    let mut names = Vec::new();
    for entry in open_listing.map_err(|e| CoreError::Parse(format!("rar: {e}")))? {
        let e = entry.map_err(|e| CoreError::Parse(format!("rar: {e}")))?;
        names.push(e.filename.to_string_lossy().to_string());
    }

    let mut archive = if password.is_empty() {
        unrar::Archive::new(path).open_for_processing()
    } else {
        unrar::Archive::with_password(path, password).open_for_processing()
    }
    .map_err(|e| CoreError::Parse(format!("rar: {e}")))?;

    let mut content = None;
    while let Some(header) = archive
        .read_header()
        .map_err(|e| CoreError::Parse(format!("rar: {e}")))?
    {
        let entry = header.entry();
        let name = entry.filename.to_string_lossy().to_string();
        let take = content.is_none() && !entry.is_directory() && (target.is_empty() || name == target);
        if take {
            let (bytes, rest) = header.read().map_err(|e| CoreError::Parse(format!("rar: {e}")))?;
            content = Some(bytes);
            archive = rest;
        } else {
            archive = header.skip().map_err(|e| CoreError::Parse(format!("rar: {e}")))?;
        }
    }
    Ok(finish(names, content))
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = inputs
            .get("archive")
            .ok_or_else(|| CoreError::MissingInput("archive".into()))?
            .as_bytes()?;
        let password = pstr(params, "password", "");
        let target = pstr(params, "entry", "");
        let fmt = pstr(params, "format", "自动");

        let kind = if fmt == "自动" {
            detect(&data)
        } else {
            fmt
        };

        match kind {
            "zip" | "ZIP" => extract_zip(&data, password, target),
            "gz" | "GZ" => extract_gz(&data),
            "tar" | "TAR" => extract_tar(&data, target),
            "7z" | "7Z" => extract_7z(&data, password, target),
            "rar" | "RAR" => extract_rar(&data, password, target),
            _ => Err(CoreError::Unsupported("未识别的压缩格式".into())),
        }
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "archive_extract",
            ARC,
            "解压",
            "#f97316",
            vec![req("archive", "压缩包字节", PortType::Bytes)],
            vec![
                req("files", "文件列表", PortType::StringList),
                opt("text", "内容", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select("format", "格式", &["自动", "zip", "7z", "rar", "gz", "tar"], "自动"),
                ParamSpec::text("password", "密码", "", false),
                ParamSpec::text("entry", "指定条目(可选)", "", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
