//! 节区提取：按名字（如 `.rodata`/`.text`/`.rsrc`）抠出某个节区的原始字节，
//! 用于取藏在节区里的数据。找不到时错误信息会列出可用节区名。
use goblin::Object;

use super::bin_common::parse;
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let want = pstr(p, "sectionName", ".rodata").trim();

        let (off, size) = match parse(&data)? {
            Object::Elf(e) => {
                let hit = e
                    .section_headers
                    .iter()
                    .find(|sh| e.shdr_strtab.get_at(sh.sh_name) == Some(want));
                match hit {
                    Some(sh) => (sh.sh_offset as usize, sh.sh_size as usize),
                    None => {
                        let avail: Vec<&str> = e
                            .section_headers
                            .iter()
                            .filter_map(|sh| e.shdr_strtab.get_at(sh.sh_name))
                            .filter(|n| !n.is_empty())
                            .collect();
                        return Err(CoreError::Other(format!(
                            "未找到节区「{want}」。可用：{}",
                            avail.join(", ")
                        )));
                    }
                }
            }
            Object::PE(pe) => {
                let hit = pe.sections.iter().find(|s| s.name().ok() == Some(want));
                match hit {
                    Some(s) => (s.pointer_to_raw_data as usize, s.size_of_raw_data as usize),
                    None => {
                        let avail: Vec<String> =
                            pe.sections.iter().map(|s| s.name().unwrap_or("").to_string()).collect();
                        return Err(CoreError::Other(format!(
                            "未找到节区「{want}」。可用：{}",
                            avail.join(", ")
                        )));
                    }
                }
            }
            _ => return Err(CoreError::Unsupported("仅支持 ELF/PE 的节区提取".into())),
        };

        // 防越界（文件被截断时夹紧）。
        let start = off.min(data.len());
        let end = off.saturating_add(size).min(data.len());
        let bytes = data[start..end].to_vec();

        let mut m = PortMap::new();
        m.insert("bytes".into(), PortValue::Bytes(Arc::from(bytes.into_boxed_slice())));
        m.insert("offset".into(), PortValue::Number(off as f64));
        m.insert("size".into(), PortValue::Number((end - start) as f64));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "section_extract",
            BIN,
            "节区提取",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("bytes", "节区字节", PortType::Bytes),
                opt("offset", "文件偏移", PortType::Number),
                opt("size", "大小", PortType::Number),
            ],
            vec![ParamSpec::text("sectionName", "节区名(如 .text/.rodata)", ".rodata", false)],
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
    fn missing_section_errors_with_names() {
        // 最小 ELF64 无节区 → 找 .rodata 报错。
        let mut e = vec![0u8; 64];
        e[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        e[4] = 2;
        e[5] = 1;
        e[6] = 1;
        e[16..18].copy_from_slice(&2u16.to_le_bytes());
        e[18..20].copy_from_slice(&0x3eu16.to_le_bytes());
        e[52..54].copy_from_slice(&64u16.to_le_bytes());
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(e.into_boxed_slice())));
        let r = GraphExecutor::run_node(
            &default_registry(),
            "section_extract",
            &i,
            &serde_json::json!({ "sectionName": ".rodata" }),
            &NullSink,
            &CancellationToken::new(),
        );
        assert!(r.is_err());
    }
}
