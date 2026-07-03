//! 压缩包文件列表：不解压就列出每个条目的名称、原始/压缩大小、压缩方式、是否加密、CRC。
//! 先读 zip 的中央目录（`by_index_raw` 对加密条目也能拿到元数据）；其它格式给出基本提示。
use std::io::Cursor;

use super::prelude::*;

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

fn list_zip(data: &[u8]) -> Result<PortMap, CoreError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(data))
        .map_err(|e| CoreError::Parse(format!("zip: {e}")))?;

    let mut names: Vec<String> = Vec::new();
    let mut rows: Vec<(String, u64, u64, String, bool, u32, bool)> = Vec::new();
    let mut enc_count = 0usize;
    for i in 0..zip.len() {
        // by_index_raw 读原始条目（不解密），加密条目也能拿到元数据。
        let e = zip
            .by_index_raw(i)
            .map_err(|e| CoreError::Parse(format!("读取条目 {i} 失败: {e}")))?;
        let name = e.name().to_string();
        let encrypted = e.encrypted();
        if encrypted {
            enc_count += 1;
        }
        names.push(name.clone());
        rows.push((
            name,
            e.size(),
            e.compressed_size(),
            format!("{:?}", e.compression()),
            encrypted,
            e.crc32(),
            e.is_dir(),
        ));
    }

    let namew = rows
        .iter()
        .map(|r| r.0.chars().count())
        .max()
        .unwrap_or(4)
        .clamp(4, 48);
    let mut text = format!("共 {} 个条目（{enc_count} 个加密）：\n", rows.len());
    for (name, size, csize, method, encrypted, crc, is_dir) in &rows {
        if *is_dir {
            text.push_str(&format!("{name:<namew$}  <目录>\n"));
            continue;
        }
        text.push_str(&format!(
            "{name:<namew$}  原始 {size:>9}  压缩 {csize:>9}  {method:<9}  {}  CRC {crc:08x}\n",
            if *encrypted { "🔒加密" } else { "明文  " },
        ));
    }

    let mut m = PortMap::new();
    m.insert("text".into(), PortValue::Text(text));
    m.insert("files".into(), PortValue::StringList(names.clone()));
    m.insert("count".into(), PortValue::Number(names.len() as f64));
    m.insert("encrypted".into(), PortValue::Bool(enc_count > 0));
    Ok(m)
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "archive")?;
        match detect(&data) {
            "zip" => list_zip(&data),
            other => Err(CoreError::Parse(format!(
                "详细文件列表目前仅支持 zip（检测到 {other}）。其它格式请用「解压」节点。"
            ))),
        }
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "archive_list",
            ARC,
            "压缩包文件列表",
            AMBER,
            vec![req("archive", "压缩包字节", PortType::Any)],
            vec![
                req("text", "列表", PortType::Text),
                opt("files", "文件名", PortType::StringList),
                opt("count", "条目数", PortType::Number),
                opt("encrypted", "含加密", PortType::Bool),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::node::PortMap;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;
    use zip::write::SimpleFileOptions;

    fn make_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            w.start_file(
                "hello.txt",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
            w.write_all(b"hello world hello world hello world").unwrap();
            w.start_file(
                "secret.txt",
                SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes256, "pass"),
            )
            .unwrap();
            w.write_all(b"top secret data").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn lists_entries_with_metadata() {
        let mut inputs = PortMap::new();
        inputs.insert(
            "archive".into(),
            PortValue::Bytes(Arc::from(make_zip().into_boxed_slice())),
        );
        let reg = default_registry();
        let out = GraphExecutor::run_node(
            &reg,
            "archive_list",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        let text = match out.get("text") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        };
        assert!(text.contains("hello.txt"), "{text}");
        assert!(
            text.contains("secret.txt") && text.contains("加密"),
            "{text}"
        );
        assert!(matches!(out.get("encrypted"), Some(PortValue::Bool(true))));
        assert!(matches!(out.get("count"), Some(PortValue::Number(n)) if *n == 2.0));
    }
}
