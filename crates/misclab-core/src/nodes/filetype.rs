//! Detect a file's type from its magic bytes.
use super::prelude::*;

// (magic bytes, human name, canonical extension without the dot).
pub(crate) const MAGICS: &[(&[u8], &str, &str)] = &[
    (&[0x89, 0x50, 0x4E, 0x47], "PNG 图片", "png"),
    (&[0xFF, 0xD8, 0xFF], "JPEG 图片", "jpg"),
    (&[0x47, 0x49, 0x46, 0x38], "GIF 图片", "gif"),
    (&[0x25, 0x50, 0x44, 0x46], "PDF 文档", "pdf"),
    (&[0x50, 0x4B, 0x03, 0x04], "ZIP / docx / jar / apk", "zip"),
    (&[0x50, 0x4B, 0x05, 0x06], "空 ZIP", "zip"),
    (&[0x50, 0x4B, 0x07, 0x08], "分卷 ZIP", "zip"),
    (&[0x52, 0x61, 0x72, 0x21], "RAR 压缩包", "rar"),
    (&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C], "7-Zip 压缩包", "7z"),
    (&[0x1F, 0x8B], "Gzip 压缩", "gz"),
    (&[0x42, 0x5A, 0x68], "Bzip2 压缩", "bz2"),
    (&[0xFD, 0x37, 0x7A, 0x58, 0x5A], "XZ 压缩", "xz"),
    (&[0x7F, 0x45, 0x4C, 0x46], "ELF 可执行", "elf"),
    (&[0x4D, 0x5A], "Windows PE / EXE / DLL", "exe"),
    (&[0x42, 0x4D], "BMP 图片", "bmp"),
    (&[0x49, 0x44, 0x33], "MP3 (ID3)", "mp3"),
    (&[0x66, 0x4C, 0x61, 0x43], "FLAC 音频", "flac"),
    (&[0x4F, 0x67, 0x67, 0x53], "OGG 音频", "ogg"),
    (&[0x53, 0x51, 0x4C, 0x69, 0x74, 0x65], "SQLite 数据库", "sqlite"),
    (&[0xCA, 0xFE, 0xBA, 0xBE], "Java class", "class"),
    (&[0x49, 0x49, 0x2A, 0x00], "TIFF (小端)", "tif"),
    (&[0x4D, 0x4D, 0x00, 0x2A], "TIFF (大端)", "tif"),
    (&[0x00, 0x61, 0x73, 0x6D], "WebAssembly", "wasm"),
    (&[0x1A, 0x45, 0xDF, 0xA3], "Matroska / WebM", "mkv"),
    (&[0x00, 0x00, 0x01, 0x00], "ICO 图标", "ico"),
    (&[0x25, 0x21, 0x50, 0x53], "PostScript", "ps"),
    (&[0xD0, 0xCF, 0x11, 0xE0], "MS Office 旧格式 (doc/xls/ppt)", "doc"),
    (&[0x38, 0x42, 0x50, 0x53], "PSD (Photoshop)", "psd"),
];

/// Returns (human name, canonical extension). The extension has no leading dot
/// and is empty when the type is unknown or has no meaningful suffix.
pub(crate) fn detect(data: &[u8]) -> (String, &'static str) {
    if data.len() >= 12 && &data[0..4] == b"RIFF" {
        let (name, ext): (&str, &str) = match &data[8..12] {
            b"WEBP" => ("WEBP 图片", "webp"),
            b"WAVE" => ("WAV 音频", "wav"),
            b"AVI " => ("AVI 视频", "avi"),
            _ => ("RIFF 容器", ""),
        };
        return (name.to_string(), ext);
    }
    if data.len() >= 12 && &data[4..8] == b"ftyp" {
        return ("MP4 / MOV (ISO 媒体)".to_string(), "mp4");
    }
    if data.len() >= 262 && &data[257..262] == b"ustar" {
        return ("TAR 归档".to_string(), "tar");
    }
    for (sig, name, ext) in MAGICS {
        if data.starts_with(sig) {
            return (name.to_string(), *ext);
        }
    }
    ("未知（未匹配已知幻数）".to_string(), "")
}

struct N;
impl Node for N {
    fn run(&self, inputs: &PortMap, _p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let (ty, ext) = detect(&data);
        let head = data.iter().take(8).map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ");
        let ext_line = if ext.is_empty() {
            "后缀名: (未知)".to_string()
        } else {
            format!("后缀名: {ext}")
        };
        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(format!("{ty}\n{ext_line}\n幻数: {head}")));
        m.insert("type".to_string(), PortValue::Text(ty));
        m.insert("ext".to_string(), PortValue::Text(ext.to_string()));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "detect_file_type",
            UTIL,
            "文件类型识别",
            AMBER,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("text", "结果", PortType::Text),
                opt("type", "类型", PortType::Text),
                opt("ext", "后缀名", PortType::Text),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::detect;

    #[test]
    fn emits_extension_for_known_types() {
        assert_eq!(detect(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]).1, "png");
        assert_eq!(detect(&[0xFF, 0xD8, 0xFF, 0xE0]).1, "jpg");
        assert_eq!(detect(&[0x50, 0x4B, 0x03, 0x04]).1, "zip");
        // RIFF containers need the sub-form (bytes 8..12) to disambiguate.
        let mut wav = b"RIFF\0\0\0\0WAVE".to_vec();
        wav.extend_from_slice(&[0u8; 4]);
        assert_eq!(detect(&wav).1, "wav");
    }

    #[test]
    fn unknown_type_has_empty_extension() {
        let (ty, ext) = detect(&[0x00, 0x11, 0x22, 0x33]);
        assert!(ty.starts_with("未知"));
        assert_eq!(ext, "");
    }
}
