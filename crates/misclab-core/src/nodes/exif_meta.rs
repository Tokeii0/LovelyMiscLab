//! Extract EXIF metadata from an image (JPEG / TIFF / HEIF / PNG / WebP).
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let reader = exif::Reader::new();
        let mut cur = std::io::Cursor::new(&data);
        let ex = match reader.read_from_container(&mut cur) {
            Ok(e) => e,
            Err(exif::Error::NotFound(_)) => {
                let mut m = PortMap::new();
                m.insert(
                    "text".to_string(),
                    PortValue::Text("未找到 EXIF 数据".to_string()),
                );
                m.insert("fields".to_string(), PortValue::StringList(Vec::new()));
                m.insert("count".to_string(), PortValue::Number(0.0));
                return Ok(m);
            }
            Err(e) => return Err(CoreError::Other(format!("EXIF 解析失败: {e}"))),
        };
        let list: Vec<String> = ex
            .fields()
            .map(|f| format!("{}: {}", f.tag, f.display_value().with_unit(&ex)))
            .collect();
        let mut m = PortMap::new();
        m.insert("text".to_string(), PortValue::Text(list.join("\n")));
        m.insert("count".to_string(), PortValue::Number(list.len() as f64));
        m.insert("fields".to_string(), PortValue::StringList(list));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "exif_extract",
            UTIL,
            "EXIF 信息",
            AMBER,
            vec![req("data", "图片", PortType::Any)],
            vec![
                req("text", "元数据", PortType::Text),
                opt("fields", "字段", PortType::StringList),
                opt("count", "数量", PortType::Number),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
