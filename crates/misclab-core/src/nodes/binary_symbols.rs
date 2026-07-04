//! 符号 / 导入 / 导出：ELF/PE 的符号表与依赖关系。`kind` 选视图：
//! 「符号」ELF 符号表；「导入」PE 导入(dll!func)/ELF 未定义动态符号+依赖库；「导出」导出符号。
use goblin::Object;
use serde_json::json;

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
        let kind = pstr(p, "kind", "导入");

        let mut lines: Vec<String> = Vec::new();
        let mut names: Vec<String> = Vec::new();
        let mut jarr: Vec<serde_json::Value> = Vec::new();

        match parse(&data)? {
            Object::Elf(e) => match kind {
                "导入" => {
                    for lib in &e.libraries {
                        lines.push(format!("[依赖库] {lib}"));
                    }
                    for s in e.dynsyms.iter().filter(|s| s.is_import()) {
                        let n = e.dynstrtab.get_at(s.st_name).unwrap_or("");
                        if n.is_empty() {
                            continue;
                        }
                        names.push(n.to_string());
                        lines.push(n.to_string());
                        jarr.push(json!({ "name": n }));
                    }
                }
                "导出" => {
                    for s in e.dynsyms.iter().filter(|s| !s.is_import() && s.is_function()) {
                        let n = e.dynstrtab.get_at(s.st_name).unwrap_or("");
                        if n.is_empty() {
                            continue;
                        }
                        names.push(n.to_string());
                        lines.push(format!("{n}  0x{:x}", s.st_value));
                        jarr.push(json!({ "name": n, "value": s.st_value }));
                    }
                }
                _ => {
                    for s in e.syms.iter() {
                        let n = e.strtab.get_at(s.st_name).unwrap_or("");
                        if n.is_empty() {
                            continue;
                        }
                        names.push(n.to_string());
                        lines.push(format!("{n}  0x{:x}{}", s.st_value, if s.is_function() { "  func" } else { "" }));
                        jarr.push(json!({ "name": n, "value": s.st_value, "func": s.is_function() }));
                    }
                }
            },
            Object::PE(pe) => match kind {
                "导出" => {
                    for ex in &pe.exports {
                        let n = ex.name.unwrap_or("");
                        names.push(n.to_string());
                        lines.push(format!("{n}  RVA 0x{:x}", ex.rva));
                        jarr.push(json!({ "name": n, "rva": ex.rva }));
                    }
                }
                _ => {
                    // 「符号」在 PE 上退化到导入（PE 无统一符号表）。
                    for im in &pe.imports {
                        let line = format!("{}!{}", im.dll, im.name);
                        names.push(line.clone());
                        lines.push(line);
                        jarr.push(json!({ "dll": im.dll, "name": im.name.as_ref(), "ordinal": im.ordinal }));
                    }
                }
            },
            Object::Mach(_) => lines.push("Mach-O 符号列举暂未支持".into()),
            Object::Archive(_) => lines.push("归档(ar) 无符号表".into()),
            _ => {}
        }

        let text = if lines.is_empty() { "（无）".to_string() } else { lines.join("\n") };
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
            "binary_symbols",
            BIN,
            "符号/导入/导出",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("text", "清单", PortType::Text),
                opt("json", "结构", PortType::Json),
                opt("names", "名称", PortType::StringList),
            ],
            vec![ParamSpec::select("kind", "类别", &["符号", "导入", "导出"], "导入")],
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
    fn minimal_elf_no_symbols() {
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
            "binary_symbols",
            &i,
            &serde_json::json!({ "kind": "导入" }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("names"), Some(PortValue::StringList(v)) if v.is_empty()));
    }

    #[test]
    fn non_executable_errors() {
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(vec![9u8, 9, 9, 9].into_boxed_slice())));
        assert!(GraphExecutor::run_node(
            &default_registry(),
            "binary_symbols",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .is_err());
    }
}
