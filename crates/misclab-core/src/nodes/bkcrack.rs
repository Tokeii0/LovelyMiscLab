//! bkcrack 节点 —— 对 ZipCrypto 传统加密的 ZIP 做**已知明文攻击**（Biham–Kocher），
//! 求出三个内部密钥并可解密目标条目。攻击算法是 [`engine`] 中 bkcrack 的原生 Rust
//! 移植（无需外部程序）。喂入已知明文可用内置的常见文件头模板，或自定义 Hex/文本。
//!
//! 已知明文对应「条目的原始数据流」：对 **Stored（未压缩）** 条目，文件头模板可直接
//! 命中；对 **Deflate** 压缩条目，应提供压缩流的已知明文（用自定义 Hex），解密结果会
//! 自动尝试 Inflate 还原。至少需要 12 字节连续明文，越多越快、解越唯一。

mod engine;

use std::io::Read;

use super::prelude::*;
use crate::progress::{LogLevel, ProgressEvent};

/// 内置明文模板：名称 → (十六进制明文, 偏移)。都是 ≥12 字节、在偏移 0 处可靠连续的
/// 文件头（适用于 Stored 条目）。
const TEMPLATES: &[(&str, &str, i32)] = &[
    // PNG 签名(8) + IHDR 块长度(0000000D) + 类型"IHDR" = 16 字节，最稳。
    ("PNG 图片", "89504e470d0a1a0a0000000d49484452", 0),
    // JFIF JPEG: FF D8 FF E0 00 10 "JFIF" 00 主版本 01 = 12 字节。
    ("JPEG 图片(JFIF)", "ffd8ffe000104a4649460001", 0),
    // OOXML(docx/xlsx/pptx): PK\x03\x04 14 00 06 00 08 00 00 00 = 12 字节。
    ("Office 文档(OOXML)", "504b03041400060008000000", 0),
    // ELF64 LSB 头 e_ident 16 字节。
    ("ELF 可执行(64位)", "7f454c46020101000000000000000000", 0),
];

/// 一个 ZIP 条目的必要信息（从中央目录 + 本地头解析而来）。
struct Entry {
    name: String,
    method: u16,
    encrypted: bool,
    /// 原始数据流（含 12 字节加密头）在文件中的起止。
    data_start: usize,
    comp_size: usize,
}

fn u16le(d: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([d[i], d[i + 1]])
}
fn u32le(d: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

/// 解析 ZIP 的中央目录，返回全部条目及其原始数据位置。
fn parse_zip(data: &[u8]) -> Result<Vec<Entry>, CoreError> {
    // 从尾部向前找 EOCD（可能有注释，故回扫）。
    let eocd = {
        let mut found = None;
        let n = data.len();
        if n >= 22 {
            let start = n - 22;
            let mut i = start as isize;
            let lower = start.saturating_sub(0xffff) as isize;
            while i >= lower {
                let p = i as usize;
                if data[p] == 0x50 && data[p + 1] == 0x4b && data[p + 2] == 0x05 && data[p + 3] == 0x06 {
                    found = Some(p);
                    break;
                }
                i -= 1;
            }
        }
        found.ok_or_else(|| CoreError::Parse("不是有效 ZIP（未找到中央目录结尾 EOCD）".into()))?
    };

    let cd_offset = u32le(data, eocd + 16) as usize;
    let mut entries = Vec::new();
    let mut i = cd_offset;
    while i + 46 <= data.len()
        && data[i] == 0x50
        && data[i + 1] == 0x4b
        && data[i + 2] == 0x01
        && data[i + 3] == 0x02
    {
        let flags = u16le(data, i + 8);
        let method = u16le(data, i + 10);
        let comp_size = u32le(data, i + 20) as usize;
        let name_len = u16le(data, i + 28) as usize;
        let extra_len = u16le(data, i + 30) as usize;
        let comment_len = u16le(data, i + 32) as usize;
        let lho = u32le(data, i + 42) as usize;
        let name = String::from_utf8_lossy(&data[i + 46..(i + 46 + name_len).min(data.len())]).into_owned();

        // 解析本地文件头以定位真实数据起点（本地头的额外字段长度可能与中央目录不同）。
        if lho + 30 <= data.len()
            && data[lho] == 0x50
            && data[lho + 1] == 0x4b
            && data[lho + 2] == 0x03
            && data[lho + 3] == 0x04
        {
            let lh_name = u16le(data, lho + 26) as usize;
            let lh_extra = u16le(data, lho + 28) as usize;
            let data_start = lho + 30 + lh_name + lh_extra;
            entries.push(Entry {
                name,
                method,
                encrypted: flags & 1 == 1,
                data_start,
                comp_size,
            });
        }

        i += 46 + name_len + extra_len + comment_len;
    }

    if entries.is_empty() {
        return Err(CoreError::Parse("ZIP 中央目录为空或无法解析".into()));
    }
    Ok(entries)
}

/// 根据参数得到已知明文（字节 + 偏移）。模板优先；否则用自定义 Hex 或文本。
fn resolve_plaintext(p: &serde_json::Value) -> Result<(Vec<u8>, i32), CoreError> {
    let tmpl = pstr(p, "template", "自定义");
    if let Some((_, hexed, off)) = TEMPLATES.iter().find(|(name, _, _)| *name == tmpl) {
        return Ok((hex::decode(hexed).expect("内置模板 Hex 合法"), *off));
    }
    let offset = pnum(p, "offset", 0.0) as i32;
    let cleaned: String = pstr(p, "plainHex", "").chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if !cleaned.is_empty() {
        let bytes = hex::decode(&cleaned)
            .map_err(|_| CoreError::Parse("已知明文 Hex 不合法（需偶数个十六进制字符）".into()))?;
        return Ok((bytes, offset));
    }
    let text = pstr(p, "plainText", "");
    if !text.is_empty() {
        return Ok((text.as_bytes().to_vec(), offset));
    }
    Err(CoreError::Other(
        "请提供已知明文：选择一个明文模板，或填写「已知明文(Hex)」/「已知明文(文本)」".into(),
    ))
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
        let entries = parse_zip(&data)?;

        // 选择密文条目：显式指定，否则自动挑第一个加密条目。
        let want = pstr(p, "cipherEntry", "").trim().to_string();
        let entry = if !want.is_empty() {
            entries
                .iter()
                .find(|e| e.name == want)
                .ok_or_else(|| CoreError::Other(format!("ZIP 中没有条目「{want}」")))?
        } else {
            entries.iter().find(|e| e.encrypted && !e.name.ends_with('/')).ok_or_else(|| {
                CoreError::Other("未指定密文条目，且未找到加密条目（是否为 ZipCrypto 加密？）".into())
            })?
        };

        if !entry.encrypted {
            return Err(CoreError::Other(format!("条目「{}」未加密，无需攻击", entry.name)));
        }
        if entry.method == 99 {
            return Err(CoreError::Other(
                "该条目是 WinZip AES 加密，不是 ZipCrypto，已知明文攻击不适用".into(),
            ));
        }
        let end = entry.data_start + entry.comp_size;
        if entry.comp_size == 0 || end > data.len() {
            return Err(CoreError::Other(format!(
                "条目「{}」数据越界或为空（可能是 ZIP64，暂不支持）",
                entry.name
            )));
        }
        let ciphertext = &data[entry.data_start..end];

        let (plain, offset) = resolve_plaintext(p)?;
        let template = pstr(p, "template", "自定义");

        let method_str = match entry.method {
            0 => "Stored",
            8 => "Deflate",
            _ => "其它压缩",
        };
        let mut report = String::new();
        report.push_str(&format!(
            "密文条目：{}（{}，密文 {} 字节）\n",
            entry.name, method_str, entry.comp_size
        ));
        report.push_str(&format!("已知明文：{} 字节 @ 偏移 {}\n", plain.len(), offset));
        if entry.method != 0 && template != "自定义" {
            report.push_str(
                "提示：该条目为压缩条目，文件头模板通常只适用于 Stored 条目；\
                 若攻击失败/很慢，请改用「自定义 Hex」提供压缩流的已知明文。\n",
            );
            ctx.log(
                LogLevel::Warn,
                "该条目为压缩(Deflate)条目：文件头明文模板通常不匹配压缩流，攻击可能无解或很慢",
            );
        }
        ctx.log(
            LogLevel::Info,
            format!("密文条目：{}（{}），已知明文 {} 字节", entry.name, method_str, plain.len()),
        );
        ctx.log(
            LogLevel::Info,
            "正在进行已知明文攻击（Biham–Kocher，多线程）…明文越多越快",
        );

        // 攻击：进度 0..0.95 由引擎回调驱动；取消经由 ctx.cancel。这些回调会被
        // 多线程调用，故只捕获可 Sync 的 sink/node/cancel 引用。
        let sink = ctx.sink;
        let node = ctx.node_id.clone();
        let cancel = ctx.cancel;
        let on_progress = |f: f32| {
            sink.emit(ProgressEvent::NodeProgress {
                node: node.clone(),
                pct: (f * 0.95).clamp(0.0, 1.0),
            });
        };
        let on_log = |m: &str| {
            sink.emit(ProgressEvent::Log {
                node: Some(node.clone()),
                level: LogLevel::Info,
                message: m.to_string(),
            });
        };
        let is_cancelled = || cancel.is_cancelled();

        let keys = engine::recover_keys(ciphertext, plain, offset, on_progress, is_cancelled, on_log)
            .map_err(|e| match e {
                engine::AttackError::Data(s) => CoreError::Other(s),
                engine::AttackError::Cancelled => CoreError::Cancelled,
                engine::AttackError::NoSolution => CoreError::Other(format!(
                    "未能求得密钥：明文/偏移可能不对，或条目 {} 的已知明文与压缩流不匹配。\n{}",
                    entry.name, report
                )),
            },
        )?;

        ctx.progress(0.96);
        let keys_str = format!("{:08x} {:08x} {:08x}", keys.x, keys.y, keys.z);
        report.push_str(&format!("\n内部密钥：{keys_str}\n"));
        ctx.log(LogLevel::Info, format!("内部密钥：{keys_str}"));

        let mut out = PortMap::new();
        out.insert("keys".into(), PortValue::Text(keys_str));

        if pbool(p, "decrypt", true) {
            let deciphered = engine::decipher(ciphertext, keys);
            // Deflate 条目解密得到的是压缩流，尝试 Inflate 还原真实文件。
            let (bytes, note) = if entry.method == 8 {
                let mut inflated = Vec::new();
                match flate2::read::DeflateDecoder::new(&deciphered[..]).read_to_end(&mut inflated) {
                    Ok(_) if !inflated.is_empty() => (inflated, "已解密并 Inflate 还原"),
                    _ => (deciphered, "已解密（Inflate 失败，输出为压缩流）"),
                }
            } else {
                (deciphered, "已解密")
            };
            report.push_str(&format!("{note}：{} 字节\n", bytes.len()));
            ctx.log(LogLevel::Info, format!("{note}，{} 字节", bytes.len()));
            out.insert("data".into(), PortValue::Bytes(Arc::from(bytes.into_boxed_slice())));
        }

        ctx.progress(1.0);
        out.insert("report".into(), PortValue::Text(report));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let template_names: Vec<&str> = std::iter::once("自定义")
        .chain(TEMPLATES.iter().map(|(n, _, _)| *n))
        .collect();
    reg.register(
        {
            let mut d = desc(
                "bkcrack",
                ARC,
                "bkcrack 明文攻击",
                AMBER,
                vec![req("archive", "加密 ZIP", PortType::Any)],
                vec![
                    req("keys", "内部密钥", PortType::Text),
                    opt("data", "解密内容", PortType::Bytes),
                    opt("report", "运行日志", PortType::Text),
                ],
                vec![
                    ParamSpec::text("cipherEntry", "密文条目名(留空自动选)", "", false),
                    ParamSpec::select("template", "明文模板", &template_names, "自定义"),
                    ParamSpec::text("plainHex", "已知明文(Hex，自定义)", "", false),
                    ParamSpec::text("plainText", "已知明文(文本，自定义)", "", false),
                    ParamSpec::number("offset", "明文偏移(字节)", -12.0, 100_000_000.0, 1.0, 0.0),
                    ParamSpec::toggle("decrypt", "求得密钥后解密该条目", true),
                ],
            );
            d.description =
                "ZipCrypto 传统加密的已知明文攻击（Biham–Kocher，bkcrack 原生移植）：求内部密钥并可解密。内置常见文件头明文模板。".into();
            d.cost = Cost::Heavy;
            d
        },
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

    fn le16(v: &mut Vec<u8>, x: u16) {
        v.extend_from_slice(&x.to_le_bytes());
    }
    fn le32(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_le_bytes());
    }

    /// Build a minimal ZIP with one Stored, ZipCrypto-encrypted entry, encrypting
    /// with the validated forward cipher so a known password produces it.
    fn make_encrypted_zip(name: &str, password: &[u8], data: &[u8]) -> Vec<u8> {
        let mut k = engine::Keys::from_password(password);
        let mut ct = Vec::new();
        for &h in &[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
            ct.push(h ^ k.get_k());
            k.update(h);
        }
        for &p in data {
            ct.push(p ^ k.get_k());
            k.update(p);
        }
        let comp = ct.len() as u32;
        let uncomp = data.len() as u32;
        let nlen = name.len() as u16;

        let mut z = Vec::new();
        // Local file header.
        z.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        le16(&mut z, 20); // version needed
        le16(&mut z, 1); // flags: bit0 encrypted
        le16(&mut z, 0); // method: stored
        le16(&mut z, 0); // time
        le16(&mut z, 0); // date
        le32(&mut z, 0); // crc (unused by parser)
        le32(&mut z, comp);
        le32(&mut z, uncomp);
        le16(&mut z, nlen);
        le16(&mut z, 0); // extra len
        z.extend_from_slice(name.as_bytes());
        z.extend_from_slice(&ct);

        let cd_offset = z.len() as u32;
        // Central directory header.
        z.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        le16(&mut z, 20); // version made by
        le16(&mut z, 20); // version needed
        le16(&mut z, 1); // flags
        le16(&mut z, 0); // method
        le16(&mut z, 0); // time
        le16(&mut z, 0); // date
        le32(&mut z, 0); // crc
        le32(&mut z, comp);
        le32(&mut z, uncomp);
        le16(&mut z, nlen);
        le16(&mut z, 0); // extra
        le16(&mut z, 0); // comment
        le16(&mut z, 0); // disk
        le16(&mut z, 0); // internal attrs
        le32(&mut z, 0); // external attrs
        le32(&mut z, 0); // local header offset
        z.extend_from_slice(name.as_bytes());
        let cd_size = z.len() as u32 - cd_offset;

        // End of central directory.
        z.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        le16(&mut z, 0); // disk
        le16(&mut z, 0); // cd start disk
        le16(&mut z, 1); // entries on disk
        le16(&mut z, 1); // total entries
        le32(&mut z, cd_size);
        le32(&mut z, cd_offset);
        le16(&mut z, 0); // comment len
        z
    }

    #[test]
    #[ignore = "runs the full attack pipeline (slow in debug, fast in release). Run: \
                `cargo test -p misclab-core --release -- --include-ignored bkcrack`."]
    fn node_recovers_and_decrypts_encrypted_zip() {
        let password = b"hunter2!";
        let data = b"flag{node_level_bkcrack_integration_ok!!}";
        let zip = make_encrypted_zip("flag.txt", password, data);

        let mut inputs = PortMap::new();
        inputs.insert("archive".into(), PortValue::Bytes(Arc::from(zip.into_boxed_slice())));
        let params = serde_json::json!({
            "cipherEntry": "flag.txt",
            "template": "自定义",
            "plainText": String::from_utf8_lossy(data),
            "decrypt": true,
        });
        let out = GraphExecutor::run_node(
            &default_registry(),
            "bkcrack",
            &inputs,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .expect("bkcrack node should run");

        let expected = engine::Keys::from_password(password);
        let expected_keys = format!("{:08x} {:08x} {:08x}", expected.x, expected.y, expected.z);
        match out.get("keys") {
            Some(PortValue::Text(s)) => assert_eq!(s, &expected_keys),
            o => panic!("keys: {o:?}"),
        }
        match out.get("data") {
            Some(PortValue::Bytes(b)) => assert_eq!(&b[..], data),
            o => panic!("data: {o:?}"),
        }
    }

    #[test]
    fn templates_are_valid_and_sufficient() {
        for (name, hexed, _) in TEMPLATES {
            let bytes = hex::decode(hexed).unwrap_or_else(|_| panic!("template {name} hex"));
            assert!(bytes.len() >= 12, "template {name} needs >= 12 bytes, got {}", bytes.len());
        }
    }

    #[test]
    fn resolves_template_and_custom() {
        let p = serde_json::json!({ "template": "PNG 图片" });
        let (bytes, off) = resolve_plaintext(&p).unwrap();
        assert_eq!(bytes, hex::decode("89504e470d0a1a0a0000000d49484452").unwrap());
        assert_eq!(off, 0);

        let p = serde_json::json!({ "template": "自定义", "plainText": "flag{demo}", "offset": 4 });
        let (bytes, off) = resolve_plaintext(&p).unwrap();
        assert_eq!(bytes, b"flag{demo}");
        assert_eq!(off, 4);
    }
}
