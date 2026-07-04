//! 可执行信息：解析 ELF / PE / Mach-O 头，概览格式/架构/位数/字节序/入口/类型等。
//! 与 `filetype`（仅看魔数）互补——这里真正解析结构。
use goblin::mach::Mach;
use goblin::Object;
use serde_json::json;

use super::bin_common::{elf_type, parse, pe_machine, pe_subsystem};
use super::prelude::*;

fn mach_arch(cputype: u32) -> &'static str {
    match cputype {
        0x0100_0007 => "x86-64",
        0x0000_0007 => "x86",
        0x0100_000c => "ARM64",
        0x0000_000c => "ARM",
        0x0100_0012 => "PowerPC64",
        0x0000_0012 => "PowerPC",
        _ => "其它",
    }
}

fn mach_info(m: Mach) -> (String, serde_json::Value) {
    match m {
        Mach::Binary(b) => {
            let arch = mach_arch(b.header.cputype);
            let bits = if b.is_64 { 64 } else { 32 };
            let endian = if b.little_endian { "小端" } else { "大端" };
            let t = format!(
                "格式：Mach-O\n架构：{arch}\n位数：{bits}\n字节序：{endian}\n入口：0x{:x}\n",
                b.entry
            );
            let j = json!({ "format": "Mach-O", "arch": arch, "bits": bits, "endian": endian, "entry": b.entry });
            (t, j)
        }
        Mach::Fat(_) => (
            "格式：Mach-O Fat/Universal（多架构）\n".to_string(),
            json!({ "format": "Mach-O Fat" }),
        ),
    }
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let (text, j) = match parse(&data)? {
            Object::Elf(e) => {
                let arch = goblin::elf::header::machine_to_str(e.header.e_machine);
                let bits = if e.is_64 { 64 } else { 32 };
                let endian = if e.little_endian { "小端" } else { "大端" };
                let ty = elf_type(e.header.e_type);
                let mut t = format!(
                    "格式：ELF\n架构：{arch}\n位数：{bits}\n字节序：{endian}\n类型：{ty}\n入口：0x{:x}\n",
                    e.entry
                );
                if let Some(interp) = e.interpreter {
                    t.push_str(&format!("解释器：{interp}\n"));
                }
                if !e.libraries.is_empty() {
                    t.push_str(&format!("依赖库：{}\n", e.libraries.join(", ")));
                }
                let j = json!({
                    "format": "ELF", "arch": arch, "bits": bits, "endian": endian, "type": ty,
                    "entry": e.entry, "interpreter": e.interpreter, "libraries": e.libraries, "isLib": e.is_lib,
                });
                (t, j)
            }
            Object::PE(p) => {
                let arch = pe_machine(p.header.coff_header.machine);
                let bits = if p.is_64 { 64 } else { 32 };
                let subsystem = p
                    .header
                    .optional_header
                    .map(|o| pe_subsystem(o.windows_fields.subsystem))
                    .unwrap_or("?");
                let ty = if p.is_lib { "DLL" } else { "EXE" };
                let mut t = format!(
                    "格式：PE\n架构：{arch}\n位数：{bits}\n类型：{ty}\n子系统：{subsystem}\n入口(RVA)：0x{:x}\n镜像基址：0x{:x}\n",
                    p.entry, p.image_base
                );
                if let Some(name) = p.name {
                    t.push_str(&format!("名称：{name}\n"));
                }
                let j = json!({
                    "format": "PE", "arch": arch, "bits": bits, "type": ty, "subsystem": subsystem,
                    "entryRva": p.entry, "imageBase": p.image_base, "isLib": p.is_lib,
                });
                (t, j)
            }
            Object::Mach(m) => mach_info(m),
            Object::Archive(a) => {
                let n = a.members().len();
                (
                    format!("格式：静态库/归档 (ar)\n成员数：{n}\n"),
                    json!({ "format": "Archive", "members": n }),
                )
            }
            _ => return Err(CoreError::Unsupported("暂不支持的可执行格式".into())),
        };

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert("json".into(), PortValue::Json(j));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "binary_info",
            BIN,
            "可执行信息",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("text", "信息", PortType::Text),
                opt("json", "结构", PortType::Json),
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

    /// 手搓最小 ELF64 头（64 字节，无节区/段），足够 goblin 解析头部。
    fn minimal_elf64() -> Vec<u8> {
        let mut e = vec![0u8; 64];
        e[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        e[4] = 2; // EI_CLASS = ELFCLASS64
        e[5] = 1; // EI_DATA = little-endian
        e[6] = 1; // EI_VERSION
        e[16..18].copy_from_slice(&2u16.to_le_bytes()); // e_type = ET_EXEC
        e[18..20].copy_from_slice(&0x3eu16.to_le_bytes()); // e_machine = EM_X86_64
        e[20..24].copy_from_slice(&1u32.to_le_bytes()); // e_version
        e[24..32].copy_from_slice(&0x401000u64.to_le_bytes()); // e_entry
        e[52..54].copy_from_slice(&64u16.to_le_bytes()); // e_ehsize
        e
    }

    fn run(data: &[u8]) -> Result<PortMap, CoreError> {
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(data.to_vec().into_boxed_slice())));
        GraphExecutor::run_node(
            &default_registry(),
            "binary_info",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
    }

    #[test]
    fn parses_minimal_elf64() {
        let out = run(&minimal_elf64()).unwrap();
        let text = match out.get("text") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        };
        assert!(text.contains("ELF"), "{text}");
        assert!(text.contains("64"), "{text}");
        assert!(text.contains("0x401000"), "{text}");
        assert!(matches!(out.get("json"), Some(PortValue::Json(_))));
    }

    #[test]
    fn non_executable_errors() {
        assert!(run(&[0x00, 0x11, 0x22, 0x33, 0x44]).is_err());
    }
}
