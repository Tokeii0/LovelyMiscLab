use super::prelude::*;
use base64::Engine as _;

fn to_bytes(v: &PortValue) -> Result<Vec<u8>, CoreError> {
    match v {
        PortValue::Bytes(b) => Ok(b.to_vec()),
        PortValue::Text(t) => Ok(t.as_bytes().to_vec()),
        PortValue::Image(url) => {
            let b64 = url.rsplit("base64,").next().unwrap_or("");
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| CoreError::Parse(format!("图片解码失败: {e}")))
        }
        other => Err(CoreError::Unsupported(format!(
            "无法保存该类型: {:?}",
            other.port_type()
        ))),
    }
}

/// Write the input value to a file under the configured default output folder.
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let value = inputs
            .get("data")
            .ok_or_else(|| CoreError::MissingInput("data".into()))?;
        let bytes = to_bytes(value)?;

        let filename = pstr(params, "filename", "output.bin");
        let filename = if filename.trim().is_empty() {
            "output.bin"
        } else {
            filename
        };
        let dir = if ctx.env.output_dir.trim().is_empty() {
            std::env::temp_dir()
        } else {
            std::path::PathBuf::from(&ctx.env.output_dir)
        };
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join(filename);
        std::fs::write(&path, &bytes).map_err(|e| CoreError::Other(format!("写入失败: {e}")))?;

        Ok(one(
            "path",
            PortValue::Text(path.to_string_lossy().to_string()),
        ))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "file_output",
            IO,
            "文件输出",
            GREEN,
            vec![req("data", "数据", PortType::Any)],
            vec![req("path", "保存路径", PortType::Text)],
            vec![ParamSpec::text("filename", "文件名", "output.bin", false)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
