//! ZIP CRC 爆破：当条目未压缩长度很小（比如把 flag 拆成每文件几字节）时，无法直接读出内容，
//! 但 ZIP 头里存了每条目的 CRC-32。这里对 size ≤ maxLen 的条目，按字符集枚举**恰好该长度**的
//! 明文，`crc32fast::hash` 命中即还原。多条目的还原字节按顺序拼接（重建被拆散的 flag）。
use std::io::Cursor;

use super::prelude::*;

const DEFAULT_CHARSET: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_{}";

/// 枚举长度恰为 `len` 的字符集组合，匹配目标 CRC-32。
fn brute(
    target: u32,
    charset: &[u8],
    len: usize,
    ctx: &mut NodeCtx,
) -> Result<Option<Vec<u8>>, CoreError> {
    if len == 0 {
        return Ok((target == 0).then(Vec::new));
    }
    let n = charset.len();
    if n == 0 {
        return Ok(None);
    }
    let mut idx = vec![0usize; len];
    let mut buf = vec![charset[0]; len];
    let mut count: u64 = 0;
    loop {
        if count.is_multiple_of(200_000) {
            ctx.check_cancel()?;
        }
        count += 1;
        if crc32fast::hash(&buf) == target {
            return Ok(Some(buf));
        }
        // 里程表进位：从最低位加 1，溢出则进位到高位。
        let mut pos = len;
        loop {
            if pos == 0 {
                return Ok(None); // 穷尽
            }
            pos -= 1;
            idx[pos] += 1;
            if idx[pos] < n {
                buf[pos] = charset[idx[pos]];
                break;
            }
            idx[pos] = 0;
            buf[pos] = charset[0];
        }
    }
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(i, "archive")?;
        let mut zip = zip::ZipArchive::new(Cursor::new(&data))
            .map_err(|e| CoreError::Parse(format!("不是有效 ZIP: {e}")))?;
        let max_len = pnum(p, "maxLen", 4.0).max(0.0) as u64;
        let cs = pstr(p, "charset", DEFAULT_CHARSET);
        let charset: Vec<u8> = cs.bytes().collect();

        let mut report = String::new();
        let mut combined: Vec<u8> = Vec::new();
        let mut hit_count = 0usize;

        for k in 0..zip.len() {
            let (name, size, crc, is_dir) = {
                let e = zip
                    .by_index_raw(k)
                    .map_err(|e| CoreError::Parse(format!("读取条目 {k} 失败: {e}")))?;
                (e.name().to_string(), e.size(), e.crc32(), e.is_dir())
            };
            if is_dir {
                continue;
            }
            if size > max_len {
                report.push_str(&format!("{name}  ({size} 字节)  跳过（超过 maxLen={max_len}）\n"));
                continue;
            }
            match brute(crc, &charset, size as usize, ctx)? {
                Some(pt) => {
                    let shown = String::from_utf8_lossy(&pt).into_owned();
                    report.push_str(&format!(
                        "{name}  ({size} 字节)  CRC {crc:08x} → \"{shown}\"\n"
                    ));
                    combined.extend_from_slice(&pt);
                    hit_count += 1;
                }
                None => {
                    report.push_str(&format!(
                        "{name}  ({size} 字节)  CRC {crc:08x} → 未在字符集内找到\n"
                    ));
                }
            }
        }

        let head = format!(
            "CRC 爆破：命中 {hit_count} 个条目，拼接明文如下（字符集 {} 字符，maxLen={max_len}）：\n",
            charset.len()
        );
        let mut m = PortMap::new();
        m.insert(
            "text".into(),
            PortValue::Text(String::from_utf8_lossy(&combined).into_owned()),
        );
        m.insert("report".into(), PortValue::Text(format!("{head}{report}")));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "zip_crc_crack",
            ARC,
            "ZIP CRC 爆破",
            AMBER,
            vec![req("archive", "ZIP", PortType::Any)],
            vec![
                req("text", "拼接明文", PortType::Text),
                opt("report", "逐条报告", PortType::Text),
            ],
            vec![
                ParamSpec::number("maxLen", "最大明文长度", 1.0, 8.0, 1.0, 4.0),
                ParamSpec::text("charset", "字符集", DEFAULT_CHARSET, false),
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
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    /// 造一个含两个 stored 短条目（"fl"、"ag"）的 zip。
    fn split_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            w.start_file("part1", opt).unwrap();
            w.write_all(b"fl").unwrap();
            w.start_file("part2", opt).unwrap();
            w.write_all(b"ag").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn recovers_split_flag() {
        let mut inputs = PortMap::new();
        inputs.insert(
            "archive".into(),
            PortValue::Bytes(Arc::from(split_zip().into_boxed_slice())),
        );
        let out = GraphExecutor::run_node(
            &default_registry(),
            "zip_crc_crack",
            &inputs,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(
            match out.get("text") {
                Some(PortValue::Text(s)) => s.clone(),
                o => panic!("{o:?}"),
            },
            "flag"
        );
    }
}
