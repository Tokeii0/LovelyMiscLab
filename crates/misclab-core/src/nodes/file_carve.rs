//! 文件雕刻（binwalk 式）：扫描输入字节里内嵌的文件签名（复用 filetype 的 MAGICS 表），
//! 列出每段 {偏移, 类型, 后缀}，并抽取某个内嵌文件（默认第 1 个非宿主文件，可选序号）。
//! 图片尾部藏 ZIP/其它文件是最常见的 CTF 套路。
use super::filetype::{detect, MAGICS};
use super::prelude::*;

/// 扫描签名命中（偏移升序）。仅用 ≥3 字节签名以降低误报；`only` 非空时按后缀集限定。
fn scan(data: &[u8], only: &[String]) -> Vec<(usize, &'static str, &'static str)> {
    let mut hits = Vec::new();
    for off in 0..data.len() {
        for (sig, name, ext) in MAGICS {
            if sig.len() >= 3 && data[off..].starts_with(sig) {
                if only.is_empty() || only.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                    hits.push((off, *name, *ext));
                }
                break; // 一个偏移只记一次（取表中先匹配者）
            }
        }
    }
    hits
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let only: Vec<String> = pstr(p, "signatures", "")
            .split([',', ' ', ';'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let all = scan(&data, &only);
        let embedded: Vec<(usize, &'static str, &'static str)> =
            all.iter().cloned().filter(|(off, _, _)| *off > 0).collect();

        let host = detect(&data).0;
        let mut text = format!("宿主：{host}\n发现 {} 个内嵌签名：\n", embedded.len());
        for (k, (off, name, ext)) in embedded.iter().enumerate() {
            text.push_str(&format!("  [{k}] 偏移 {off} (0x{off:X})  {name}  .{ext}\n"));
        }
        if embedded.is_empty() {
            text.push_str("（未发现内嵌文件；纯 LSB/位平面隐写请用对应节点）\n");
        }

        let mut m = PortMap::new();
        m.insert("count".into(), PortValue::Number(embedded.len() as f64));
        if !embedded.is_empty() {
            let idx = (pnum(p, "index", 0.0).max(0.0) as usize).min(embedded.len() - 1);
            let (start, _, ext) = embedded[idx];
            // 结束偏移 = 下一个（任意）命中，或 EOF。
            let end = all
                .iter()
                .map(|(o, _, _)| *o)
                .filter(|&o| o > start)
                .min()
                .unwrap_or(data.len());
            let carved = data[start..end].to_vec();
            text.push_str(&format!(
                "\n已抽取 [{idx}] 偏移 {start} 起 {} 字节（.{ext}）。",
                end - start
            ));
            m.insert(
                "bytes".into(),
                PortValue::Bytes(Arc::from(carved.into_boxed_slice())),
            );
        }
        m.insert("text".into(), PortValue::Text(text));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "file_carve",
            UTIL,
            "文件雕刻(binwalk)",
            AMBER,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("text", "命中清单", PortType::Text),
                opt("bytes", "抽取的文件", PortType::Bytes),
                opt("count", "内嵌数", PortType::Number),
            ],
            vec![
                ParamSpec::number("index", "抽取第几个(0基)", 0.0, 64.0, 1.0, 0.0),
                ParamSpec::text("signatures", "限定后缀(逗号分隔,空=全部)", "", false),
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

    #[test]
    fn carves_appended_zip() {
        // PNG 头 + 填充 + 尾附 ZIP。
        let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0u8; 16]);
        let zip_off = data.len();
        data.extend_from_slice(b"PK\x03\x04\x14\x00\x00\x00hidden-zip-body");

        let mut inputs = PortMap::new();
        inputs.insert(
            "data".into(),
            PortValue::Bytes(Arc::from(data.into_boxed_slice())),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "file_carve",
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
        assert!(text.contains("zip"), "{text}");
        assert!(text.contains(&format!("偏移 {zip_off}")), "{text}");
        assert!(matches!(out.get("count"), Some(PortValue::Number(n)) if *n == 1.0));
        let bytes = match out.get("bytes") {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("{o:?}"),
        };
        assert!(bytes.starts_with(b"PK\x03\x04"));
    }
}
