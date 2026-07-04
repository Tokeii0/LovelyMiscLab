//! 字符串提取（strings）：扫描字节里连续可打印字符 ≥ 最小长度的片段，支持 ASCII 与
//! UTF-16LE。对任意字节都可用（不限可执行文件），逆向/取证的起手动作。
use super::prelude::*;

#[inline]
fn printable(b: u8) -> bool {
    (0x20..=0x7e).contains(&b)
}

/// 连续 ASCII 可打印片段 → (偏移, 字符串)。
fn scan_ascii(data: &[u8], min: usize) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut cur = String::new();
    for (i, &b) in data.iter().enumerate() {
        if printable(b) {
            if cur.is_empty() {
                start = i;
            }
            cur.push(b as char);
        } else if cur.len() >= min {
            out.push((start, std::mem::take(&mut cur)));
        } else {
            cur.clear();
        }
    }
    if cur.len() >= min {
        out.push((start, cur));
    }
    out
}

/// 连续 UTF-16LE 可打印片段（可打印字节 + 0x00 重复）→ (偏移, 字符串)。
fn scan_utf16le(data: &[u8], min: usize) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut cur = String::new();
    let mut i = 0usize;
    while i + 1 < data.len() {
        if data[i + 1] == 0 && printable(data[i]) {
            if cur.is_empty() {
                start = i;
            }
            cur.push(data[i] as char);
            i += 2;
        } else {
            if cur.len() >= min {
                out.push((start, std::mem::take(&mut cur)));
            } else {
                cur.clear();
            }
            i += 1;
        }
    }
    if cur.len() >= min {
        out.push((start, cur));
    }
    out
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
        let min = (pnum(p, "minLen", 4.0).max(1.0)) as usize;
        let enc = pstr(p, "encoding", "ASCII");
        let show_off = pbool(p, "showOffset", false);

        let mut hits: Vec<(usize, String)> = Vec::new();
        if enc == "ASCII" || enc == "两者" {
            hits.extend(scan_ascii(&data, min));
        }
        if enc == "UTF-16LE" || enc == "两者" {
            hits.extend(scan_utf16le(&data, min));
        }
        if enc == "两者" {
            hits.sort_by_key(|(o, _)| *o);
        }

        let strings: Vec<String> = hits.iter().map(|(_, s)| s.clone()).collect();
        let text = if show_off {
            hits.iter()
                .map(|(o, s)| format!("{o:#010x}  {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            strings.join("\n")
        };
        let count = strings.len() as f64;

        let mut m = PortMap::new();
        m.insert("strings".into(), PortValue::StringList(strings));
        m.insert("text".into(), PortValue::Text(text));
        m.insert("count".into(), PortValue::Number(count));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "strings",
            BIN,
            "字符串提取",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("strings", "字符串", PortType::StringList),
                opt("text", "文本", PortType::Text),
                opt("count", "数量", PortType::Number),
            ],
            vec![
                ParamSpec::number("minLen", "最小长度", 1.0, 64.0, 1.0, 4.0),
                ParamSpec::select("encoding", "编码", &["ASCII", "UTF-16LE", "两者"], "ASCII"),
                ParamSpec::toggle("showOffset", "显示偏移", false),
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

    fn strings_of(data: &[u8], enc: &str, min: f64) -> Vec<String> {
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(data.to_vec().into_boxed_slice())));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "strings",
            &i,
            &serde_json::json!({ "encoding": enc, "minLen": min }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("strings") {
            Some(PortValue::StringList(v)) => v.clone(),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn ascii_runs() {
        assert_eq!(strings_of(b"AB\x00hello\x00\x01world!", "ASCII", 4.0), vec!["hello", "world!"]);
        // "AB" (len 2) filtered out by min 4
        assert!(strings_of(b"AB\x00", "ASCII", 4.0).is_empty());
    }

    #[test]
    fn utf16le_runs() {
        // "flag" in UTF-16LE
        let data = b"f\x00l\x00a\x00g\x00\xff\xff";
        assert_eq!(strings_of(data, "UTF-16LE", 4.0), vec!["flag"]);
    }
}
