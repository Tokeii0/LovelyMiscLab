use base64::Engine as _;
use serde_json::Value;

use super::prelude::*;

fn entries(bundle: &Value) -> Result<&Vec<Value>, CoreError> {
    bundle
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CoreError::Parse("解压结果缺少 entries 数组".into()))
}

fn is_dir(entry: &Value) -> bool {
    entry
        .get("isDir")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn name(entry: &Value) -> &str {
    entry.get("name").and_then(|v| v.as_str()).unwrap_or("")
}

fn select_entry<'a>(
    items: &'a [Value],
    target: &str,
    index: usize,
    mode: &str,
) -> Result<&'a Value, CoreError> {
    let files = items
        .iter()
        .filter(|entry| !is_dir(entry))
        .collect::<Vec<_>>();
    let target = target.trim();

    if target.is_empty() {
        return files
            .get(index)
            .copied()
            .ok_or_else(|| CoreError::Parse(format!("解压结果中没有第 {index} 个文件")));
    }

    let matched = match mode {
        "包含" => files
            .iter()
            .copied()
            .find(|entry| name(entry).contains(target)),
        "正则" => {
            let re = regex::Regex::new(target)
                .map_err(|e| CoreError::Parse(format!("文件名正则无效: {e}")))?;
            files.iter().copied().find(|entry| re.is_match(name(entry)))
        }
        _ => files.iter().copied().find(|entry| name(entry) == target),
    };

    matched.ok_or_else(|| {
        let preview = files
            .iter()
            .map(|entry| name(entry).to_string())
            .take(8)
            .collect::<Vec<_>>()
            .join(", ");
        CoreError::Parse(format!("找不到解压文件: {target}。可选文件: {preview}"))
    })
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let bundle = match inputs.get("entries") {
            Some(PortValue::Json(v)) => v,
            Some(other) => {
                return Err(CoreError::Type(format!(
                    "expected Json, got {:?}",
                    other.port_type()
                )));
            }
            None => return Err(CoreError::MissingInput("entries".into())),
        };

        let index = pnum(params, "index", 0.0).max(0.0) as usize;
        let target = pstr(params, "entry", "");
        let mode = pstr(params, "match", "精确");
        let entry = select_entry(entries(bundle)?, target, index, mode)?;
        if let Some(error) = entry.get("error").and_then(|v| v.as_str()) {
            if !error.is_empty() {
                return Err(CoreError::Parse(format!(
                    "文件 {} 无法读取: {error}",
                    name(entry)
                )));
            }
        }

        let encoded = entry
            .get("dataBase64")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                let why = if entry.get("dataOmitted").and_then(|v| v.as_bool()) == Some(true) {
                    "解压结果过大未内嵌数据，请在「解压」节点的「指定条目」里直接选择它"
                } else {
                    "没有已解包数据"
                };
                CoreError::Parse(format!("文件 {}：{why}", name(entry)))
            })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| CoreError::Parse(format!("解压文件 base64 损坏: {e}")))?;
        let filename = name(entry).to_string();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let mut out = PortMap::new();
        out.insert("name".into(), PortValue::Text(filename));
        out.insert("text".into(), PortValue::Text(text));
        out.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(bytes.clone().into_boxed_slice())),
        );
        out.insert("size".into(), PortValue::Number(bytes.len() as f64));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "archive_get_file",
            ARC,
            "取解压文件",
            AMBER,
            vec![req("entries", "解压结果", PortType::Json)],
            vec![
                req("bytes", "文件字节", PortType::Bytes),
                opt("text", "文件文本", PortType::Text),
                opt("name", "文件名", PortType::Text),
                opt("size", "大小", PortType::Number),
            ],
            vec![
                ParamSpec::text("entry", "文件名(可选)", "", false),
                ParamSpec::select("match", "匹配", &["精确", "包含", "正则"], "精确"),
                ParamSpec::number("index", "文件序号", 0.0, 100000.0, 1.0, 0.0),
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
    use serde_json::json;

    fn run(bundle: Value, params: Value) -> PortMap {
        let mut inputs = PortMap::new();
        inputs.insert("entries".into(), PortValue::Json(bundle));
        GraphExecutor::run_node(
            &default_registry(),
            "archive_get_file",
            &inputs,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    #[test]
    fn gets_file_by_index_or_name() {
        let bundle = json!({
            "kind": "misclab.archive.entries",
            "version": 1,
            "format": "zip",
            "entries": [
                { "name": "a.txt", "isDir": false, "dataBase64": "YWxwaGE=", "size": 5 },
                { "name": "dir/b.bin", "isDir": false, "dataBase64": "AAECAw==", "size": 4 }
            ]
        });
        let out = run(bundle.clone(), json!({ "index": 1 }));
        assert!(matches!(out.get("name"), Some(PortValue::Text(s)) if s == "dir/b.bin"));
        assert!(
            matches!(out.get("bytes"), Some(PortValue::Bytes(b)) if b.as_ref() == [0, 1, 2, 3])
        );

        let out = run(bundle, json!({ "entry": "b.bin", "match": "包含" }));
        assert!(matches!(out.get("name"), Some(PortValue::Text(s)) if s == "dir/b.bin"));
    }
}
