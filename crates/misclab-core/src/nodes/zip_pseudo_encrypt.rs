//! 压缩包伪加密（出题）：与「压缩包伪加密修复」[`super::zip_repair`] 相反 —— 把 ZIP 每个
//! 条目的通用位标志加密位(bit0) 置 1，制造「需要密码」的假象，但数据其实没加密（解压软件
//! 会提示输入密码，去掉该位即可正常解压）。走 EOCD→中央目录→本地文件头 的精确路径，避免
//! 误伤压缩数据里的 `PK` 序列。用于出 misc 题：给一个「看似加密」其实伪加密的压缩包。
use super::prelude::*;

fn u16le(d: &[u8], o: usize) -> usize {
    d[o] as usize | (d[o + 1] as usize) << 8
}
fn u32le(d: &[u8], o: usize) -> usize {
    u16le(d, o) | u16le(d, o + 2) << 16
}

/// 定位 EOCD（`PK\x05\x06`），从尾部往前找（注释最长 65535 字节）。
fn find_eocd(d: &[u8]) -> Option<usize> {
    if d.len() < 22 {
        return None;
    }
    let start = d.len().saturating_sub(22 + 65535);
    (start..=d.len() - 22)
        .rev()
        .find(|&i| &d[i..i + 4] == b"PK\x05\x06")
}

struct N;
impl Node for N {
    fn run(&self, i: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let mut d = in_bytes(i, "data")?;
        if !(d.starts_with(b"PK\x03\x04")
            || d.starts_with(b"PK\x05\x06")
            || d.starts_with(b"PK\x01\x02"))
        {
            return Err(CoreError::Parse("不是 ZIP 文件（缺少 PK 头）。".into()));
        }
        let eocd = find_eocd(&d)
            .ok_or_else(|| CoreError::Parse("找不到 ZIP 中央目录（EOCD），文件可能截断。".into()))?;
        let count = u16le(&d, eocd + 10);
        let cd_off = u32le(&d, eocd + 16);

        let set_strong = pbool(p, "setStrong", false);
        let bits: u16 = if set_strong { 0x0001 | 0x0040 } else { 0x0001 };

        let mut cur = cd_off;
        let mut marked = 0usize;
        let mut scanned = 0usize;
        for _ in 0..count {
            if cur + 46 > d.len() || &d[cur..cur + 4] != b"PK\x01\x02" {
                return Err(CoreError::Parse(format!(
                    "中央目录第 {scanned} 条解析失败（可能是 ZIP64 或文件损坏）。"
                )));
            }
            scanned += 1;
            let n = u16le(&d, cur + 28); // 文件名长度
            let m = u16le(&d, cur + 30); // 扩展字段长度
            let k = u16le(&d, cur + 32); // 注释长度
            let lfh_off = u32le(&d, cur + 42); // 对应本地文件头偏移

            // 中央目录头通用位标志 @ +8
            let cd_flag = u16le(&d, cur + 8) as u16;
            let new_cd = cd_flag | bits;
            if new_cd != cd_flag {
                d[cur + 8..cur + 10].copy_from_slice(&new_cd.to_le_bytes());
                marked += 1;
            }
            // 本地文件头通用位标志 @ +6
            if lfh_off + 8 <= d.len() && &d[lfh_off..lfh_off + 4] == b"PK\x03\x04" {
                let lf = u16le(&d, lfh_off + 6) as u16;
                let new_lf = lf | bits;
                if new_lf != lf {
                    d[lfh_off + 6..lfh_off + 8].copy_from_slice(&new_lf.to_le_bytes());
                }
            }
            cur += 46 + n + m + k;
        }

        let report = format!(
            "伪加密完成：{scanned} 个条目中标记了 {marked} 个加密位{}。用「压缩包伪加密修复」节点即可还原。",
            if set_strong { "（含强加密位 bit6）" } else { "" }
        );
        let mut out = PortMap::new();
        out.insert("bytes".into(), PortValue::Bytes(Arc::from(d.into_boxed_slice())));
        out.insert("report".into(), PortValue::Text(report));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "zip_pseudo_encrypt",
            ARC,
            "压缩包伪加密(出题)",
            AMBER,
            vec![req("data", "ZIP", PortType::Any)],
            vec![
                req("bytes", "伪加密后字节", PortType::Bytes),
                opt("report", "分析", PortType::Text),
            ],
            vec![ParamSpec::toggle("setStrong", "同时置强加密位(bit6)", false)],
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
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    fn plain_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            w.start_file(
                "flag.txt",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            w.write_all(b"flag{make_it_look_locked}").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn marks_then_repair_round_trips() {
        // A plain zip is not encrypted.
        let plain = plain_zip();
        assert!(!zip::ZipArchive::new(Cursor::new(&plain))
            .unwrap()
            .by_index_raw(0)
            .unwrap()
            .encrypted());

        // Pseudo-encrypt it → the zip now reports "encrypted".
        let mut inputs = PortMap::new();
        inputs.insert("data".into(), PortValue::Bytes(Arc::from(plain.into_boxed_slice())));
        let reg = default_registry();
        let out = GraphExecutor::run_node(
            &reg,
            "zip_pseudo_encrypt",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        let faked = match out.get("bytes") {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("{o:?}"),
        };
        assert!(zip::ZipArchive::new(Cursor::new(&faked))
            .unwrap()
            .by_index_raw(0)
            .unwrap()
            .encrypted());

        // zip_repair undoes it — the inverse round-trips cleanly.
        let mut ins2 = PortMap::new();
        ins2.insert("data".into(), PortValue::Bytes(Arc::from(faked.into_boxed_slice())));
        let out2 = GraphExecutor::run_node(
            &reg,
            "zip_repair",
            &ins2,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        let fixed = match out2.get("bytes") {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("{o:?}"),
        };
        let mut a = zip::ZipArchive::new(Cursor::new(&fixed)).unwrap();
        assert!(!a.by_index_raw(0).unwrap().encrypted());
        use std::io::Read as _;
        let mut s = String::new();
        a.by_index(0).unwrap().read_to_string(&mut s).unwrap();
        assert_eq!(s, "flag{make_it_look_locked}");
    }
}
