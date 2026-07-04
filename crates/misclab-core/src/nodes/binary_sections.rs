//! 节区列表：列出 ELF/PE 的节区（名称/虚拟地址/大小/文件偏移/权限）。
//! `text` 是等宽表，`json` 是结构化数组，`names` 是名称列表（可喂 filter_list/switch_case）。
use goblin::Object;
use serde_json::json;

use super::bin_common::{parse, pe_perms};
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        // 行：(名称, 虚拟地址, 大小, 文件偏移, 权限)
        let mut rows: Vec<(String, u64, u64, u64, String)> = Vec::new();
        let mut jarr: Vec<serde_json::Value> = Vec::new();
        let mut note = String::new();

        match parse(&data)? {
            Object::Elf(e) => {
                for sh in &e.section_headers {
                    let name = e.shdr_strtab.get_at(sh.sh_name).unwrap_or("").to_string();
                    let perms = format!(
                        "{}{}{}",
                        if sh.is_alloc() { 'a' } else { '-' },
                        if sh.is_writable() { 'w' } else { '-' },
                        if sh.is_executable() { 'x' } else { '-' },
                    );
                    rows.push((name.clone(), sh.sh_addr, sh.sh_size, sh.sh_offset, perms.clone()));
                    jarr.push(json!({
                        "name": name, "addr": sh.sh_addr, "size": sh.sh_size,
                        "offset": sh.sh_offset, "perms": perms,
                    }));
                }
            }
            Object::PE(p) => {
                for s in &p.sections {
                    let name = s.name().unwrap_or("").to_string();
                    let perms = pe_perms(s.characteristics);
                    rows.push((
                        name.clone(),
                        s.virtual_address as u64,
                        s.virtual_size as u64,
                        s.pointer_to_raw_data as u64,
                        perms.clone(),
                    ));
                    jarr.push(json!({
                        "name": name, "vaddr": s.virtual_address, "vsize": s.virtual_size,
                        "offset": s.pointer_to_raw_data, "rawSize": s.size_of_raw_data, "perms": perms,
                    }));
                }
            }
            Object::Mach(_) => note = "Mach-O 节区列举暂未支持（可用 binary_info / strings）".into(),
            Object::Archive(_) => note = "归档(ar) 无节区概念".into(),
            _ => note = "该格式暂不支持节区列举".into(),
        }

        let text = if rows.is_empty() {
            if note.is_empty() { "（无节区）".into() } else { note }
        } else {
            let mut t = format!("{:<16}{:>12}{:>12}{:>12}  权限\n", "名称", "虚拟地址", "大小", "文件偏移");
            for (name, addr, size, off, perms) in &rows {
                t.push_str(&format!("{name:<16}{addr:>#12x}{size:>12}{off:>#12x}  {perms}\n"));
            }
            t
        };
        let names: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert("json".into(), PortValue::Json(json!(jarr)));
        m.insert("names".into(), PortValue::StringList(names));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "binary_sections",
            BIN,
            "节区列表",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("text", "节区表", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("names", "名称", PortType::StringList),
            ],
            vec![],
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
    fn minimal_elf_has_no_sections() {
        // 复用 binary_info 的最小 ELF64（无节区）——应优雅返回空表，不报错。
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
        let out = GraphExecutor::run_node(
            &default_registry(),
            "binary_sections",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("names"), Some(PortValue::StringList(v)) if v.is_empty()));
    }

    #[test]
    fn non_executable_errors() {
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(vec![0u8, 1, 2, 3].into_boxed_slice())));
        assert!(GraphExecutor::run_node(
            &default_registry(),
            "binary_sections",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .is_err());
    }
}
