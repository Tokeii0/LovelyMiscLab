//! 创建 ZIP 压缩包（出题）：把一段数据（字节或文本）打包成一个单文件 ZIP。可选内部文件名
//! 与压缩方式（Deflated / Stored）。常用于出 misc 题：把 flag 塞进压缩包，再接
//! 「压缩包伪加密(出题)」[`super::zip_pseudo_encrypt`] 做伪加密题，或直接给出压缩包。
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use super::prelude::*;

struct N;
impl Node for N {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let filename = {
            let f = pstr(p, "filename", "flag.txt");
            if f.trim().is_empty() { "flag.txt" } else { f }
        };
        let method_name = pstr(p, "method", "Deflated");
        let method = match method_name {
            "Stored" => CompressionMethod::Stored,
            _ => CompressionMethod::Deflated,
        };

        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default().compression_method(method);
            w.start_file(filename, opts)
                .map_err(|e| CoreError::Other(format!("创建 ZIP 条目失败: {e}")))?;
            w.write_all(&data)
                .map_err(|e| CoreError::Other(format!("写入 ZIP 失败: {e}")))?;
            w.finish()
                .map_err(|e| CoreError::Other(format!("完成 ZIP 失败: {e}")))?;
        }

        let report = format!(
            "已打包 {} 字节到「{filename}」（{method_name}），ZIP 共 {} 字节。",
            data.len(),
            buf.len()
        );
        let mut out = PortMap::new();
        out.insert("bytes".into(), PortValue::Bytes(Arc::from(buf.into_boxed_slice())));
        out.insert("report".into(), PortValue::Text(report));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "zip_create",
            ARC,
            "创建压缩包",
            AMBER,
            vec![req("data", "文件内容", PortType::Any)],
            vec![
                req("bytes", "ZIP 字节", PortType::Bytes),
                opt("report", "分析", PortType::Text),
            ],
            vec![
                ParamSpec::text("filename", "内部文件名", "flag.txt", false),
                ParamSpec::select("method", "压缩方式", &["Deflated", "Stored"], "Deflated"),
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
    use std::io::{Cursor, Read};

    #[test]
    fn packs_data_into_readable_zip() {
        let mut inputs = PortMap::new();
        inputs.insert("data".into(), PortValue::Text("flag{zip_it}".into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "zip_create",
            &inputs,
            &serde_json::json!({ "filename": "secret.txt", "method": "Stored" }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        let zip_bytes = match out.get("bytes") {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("{o:?}"),
        };
        // The produced bytes are a valid ZIP whose single entry round-trips.
        let mut a = zip::ZipArchive::new(Cursor::new(&zip_bytes)).unwrap();
        assert_eq!(a.len(), 1);
        let mut f = a.by_index(0).unwrap();
        assert_eq!(f.name(), "secret.txt");
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        assert_eq!(s, "flag{zip_it}");
    }
}
