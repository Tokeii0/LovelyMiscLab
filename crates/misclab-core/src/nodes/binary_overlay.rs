//! 附加数据 / overlay：真二进制（所有节区文件数据）末尾之后的附加字节。把真文件后面
//! 追加数据当藏点是超常见 CTF 套路（`copy /b a.exe+secret`、图后附 zip 的可执行版）。
use goblin::Object;

use super::bin_common::parse;
use super::prelude::*;

const SHT_NOBITS: u32 = 8; // .bss 等不占文件空间的节区

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "data")?;
        let end = match parse(&data)? {
            Object::Elf(e) => e
                .section_headers
                .iter()
                .filter(|sh| sh.sh_type != SHT_NOBITS)
                .map(|sh| (sh.sh_offset + sh.sh_size) as usize)
                .max()
                .unwrap_or(0),
            Object::PE(pe) => pe
                .sections
                .iter()
                .map(|s| (s.pointer_to_raw_data + s.size_of_raw_data) as usize)
                .max()
                .unwrap_or(0),
            _ => return Err(CoreError::Unsupported("仅支持 ELF/PE 的 overlay 检测".into())),
        };
        let end = end.min(data.len());
        let overlay = if end < data.len() { data[end..].to_vec() } else { Vec::new() };
        let size = overlay.len();

        let mut m = PortMap::new();
        m.insert("bytes".into(), PortValue::Bytes(Arc::from(overlay.into_boxed_slice())));
        m.insert("offset".into(), PortValue::Number(end as f64));
        m.insert("size".into(), PortValue::Number(size as f64));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "binary_overlay",
            BIN,
            "附加数据/overlay",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("bytes", "附加数据", PortType::Bytes),
                opt("offset", "起始偏移", PortType::Number),
                opt("size", "大小", PortType::Number),
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
    fn non_executable_errors() {
        let mut i = PortMap::new();
        i.insert("data".into(), PortValue::Bytes(Arc::from(vec![1u8, 2, 3, 4].into_boxed_slice())));
        assert!(GraphExecutor::run_node(
            &default_registry(),
            "binary_overlay",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .is_err());
    }
}
