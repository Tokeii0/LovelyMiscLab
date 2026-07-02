//! Compress — Gzip, Zlib, raw Deflate (via flate2). Decompression / archive
//! extraction is handled by the `archive_extract` node ("解压").
use std::io::Write;

use flate2::Compression;

use super::prelude::*;

fn io<E: std::fmt::Display>(e: E) -> CoreError {
    CoreError::Other(format!("压缩失败: {e}"))
}

struct Comp;
impl Node for Comp {
    fn run(&self, inputs: &PortMap, params: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let out = match pstr(params, "format", "Gzip") {
            "Zlib" => {
                let mut e = flate2::write::ZlibEncoder::new(Vec::new(), Compression::default());
                e.write_all(&data).map_err(io)?;
                e.finish().map_err(io)?
            }
            "Raw Deflate" => {
                let mut e = flate2::write::DeflateEncoder::new(Vec::new(), Compression::default());
                e.write_all(&data).map_err(io)?;
                e.finish().map_err(io)?
            }
            _ => {
                let mut e = flate2::write::GzEncoder::new(Vec::new(), Compression::default());
                e.write_all(&data).map_err(io)?;
                e.finish().map_err(io)?
            }
        };
        let mut m = PortMap::new();
        m.insert("hex".to_string(), PortValue::Text(hex::encode(&out)));
        m.insert("bytes".to_string(), PortValue::Bytes(Arc::from(out.into_boxed_slice())));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "compress",
            ARC,
            "压缩",
            AMBER,
            vec![req("data", "输入", PortType::Any)],
            vec![req("hex", "hex", PortType::Text), opt("bytes", "字节", PortType::Bytes)],
            vec![ParamSpec::select("format", "格式", &["Gzip", "Zlib", "Raw Deflate"], "Gzip")],
        ),
        Arc::new(|| Arc::new(Comp)),
    );
}
