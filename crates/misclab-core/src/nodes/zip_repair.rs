//! 压缩包伪加密修复：ZIP「伪加密」是把通用位标志的加密位（bit0）置 1，但数据其实没加密，
//! 于是解压软件误报「需要密码」。本节点从 EOCD 定位中央目录，逐条清掉**中央目录头(+8)**
//! 与其对应**本地文件头(+6)**的加密位，输出修复后的字节。可选连带清「强加密位」(bit6)。
//! 走 EOCD→中央目录→本地头偏移的精确路径，避免扫描字节流误伤压缩数据里的 `PK` 序列。
//! RAR 结构复杂，仅检测提示、不修复。
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
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut d = in_bytes(i, "data")?;
        if d.starts_with(b"Rar!") {
            return Err(CoreError::Parse(
                "检测到 RAR。RAR 伪加密结构复杂，本节点暂不修复；请用「解压」节点或专门工具处理。"
                    .into(),
            ));
        }
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

        let clear_strong = pbool(p, "clearStrong", false);
        let mask: u16 = if clear_strong {
            !(0x0001 | 0x0040)
        } else {
            !0x0001
        };

        let mut cur = cd_off;
        let mut cleared = 0usize;
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
            let new_cd = cd_flag & mask;
            if new_cd != cd_flag {
                d[cur + 8..cur + 10].copy_from_slice(&new_cd.to_le_bytes());
                cleared += 1;
            }
            // 本地文件头通用位标志 @ +6
            if lfh_off + 8 <= d.len() && &d[lfh_off..lfh_off + 4] == b"PK\x03\x04" {
                let lf = u16le(&d, lfh_off + 6) as u16;
                let new_lf = lf & mask;
                if new_lf != lf {
                    d[lfh_off + 6..lfh_off + 8].copy_from_slice(&new_lf.to_le_bytes());
                }
            }
            cur += 46 + n + m + k;
        }

        let report = if cleared == 0 {
            format!("扫描 {scanned} 个条目，未发现伪加密位（没有条目置了加密位），文件未改动。")
        } else {
            format!(
                "修复完成：{scanned} 个条目中清除了 {cleared} 个加密位{}。",
                if clear_strong { "（含强加密位 bit6）" } else { "" }
            )
        };
        let mut out = PortMap::new();
        out.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(d.into_boxed_slice())),
        );
        out.insert("report".into(), PortValue::Text(report));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "zip_repair",
            ARC,
            "压缩包伪加密修复",
            AMBER,
            vec![req("data", "ZIP", PortType::Any)],
            vec![
                req("bytes", "修复后字节", PortType::Bytes),
                opt("report", "分析", PortType::Text),
            ],
            vec![ParamSpec::toggle("clearStrong", "同时清强加密位(bit6)", false)],
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
    use std::io::{Cursor, Read, Write};
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
            w.write_all(b"flag{fake_encryption}").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    /// 把所有 LFH(+6)/CDH(+8) 通用位标志置上 bit0，伪造「加密」。
    fn set_fake_encryption(d: &mut [u8]) {
        let mut i = 0;
        while i + 10 <= d.len() {
            if &d[i..i + 4] == b"PK\x03\x04" {
                let f = u16le(d, i + 6) as u16 | 1;
                d[i + 6..i + 8].copy_from_slice(&f.to_le_bytes());
            } else if &d[i..i + 4] == b"PK\x01\x02" {
                let f = u16le(d, i + 8) as u16 | 1;
                d[i + 8..i + 10].copy_from_slice(&f.to_le_bytes());
            }
            i += 1;
        }
    }

    #[test]
    fn clears_fake_encryption() {
        let mut z = plain_zip();
        set_fake_encryption(&mut z);
        // 伪加密后 zip 认为是加密的。
        {
            let mut a = zip::ZipArchive::new(Cursor::new(&z)).unwrap();
            assert!(a.by_index_raw(0).unwrap().encrypted());
        }
        let mut inputs = PortMap::new();
        inputs.insert(
            "data".into(),
            PortValue::Bytes(Arc::from(z.into_boxed_slice())),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "zip_repair",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        let fixed = match out.get("bytes") {
            Some(PortValue::Bytes(b)) => b.to_vec(),
            o => panic!("{o:?}"),
        };
        let mut a = zip::ZipArchive::new(Cursor::new(&fixed)).unwrap();
        assert!(!a.by_index_raw(0).unwrap().encrypted());
        let mut s = String::new();
        a.by_index(0).unwrap().read_to_string(&mut s).unwrap();
        assert_eq!(s, "flag{fake_encryption}");
    }
}
