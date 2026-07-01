use super::prelude::*;

/// Load a file chosen with the file picker; exposes its bytes, text, path, size.
struct N;
impl Node for N {
    fn run(
        &self,
        _inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let path = pstr(params, "path", "");
        if path.trim().is_empty() {
            return Err(CoreError::Other("未选择文件".into()));
        }
        let bytes = std::fs::read(path).map_err(|e| CoreError::Other(format!("读取失败: {e}")))?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let size = bytes.len();

        let mut out = PortMap::new();
        out.insert("bytes".to_string(), PortValue::Bytes(Arc::from(bytes.into_boxed_slice())));
        out.insert("text".to_string(), PortValue::Text(text));
        out.insert("path".to_string(), PortValue::Text(path.to_string()));
        out.insert("size".to_string(), PortValue::Number(size as f64));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "file_import",
            IO,
            "文件导入",
            SLATE,
            vec![],
            vec![
                req("bytes", "字节", PortType::Bytes),
                opt("text", "文本", PortType::Text),
                opt("path", "路径", PortType::Text),
                opt("size", "大小", PortType::Number),
            ],
            vec![ParamSpec::file("path", "文件")],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
