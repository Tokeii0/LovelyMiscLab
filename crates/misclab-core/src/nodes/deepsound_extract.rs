//! Extract secret files hidden by **DeepSound** (Jpinsoft) from a WAV carrier.
//!
//! Format reverse-engineered from the official `Steganography.dll` and verified
//! byte-exact against real DeepSound output (plain + AES). DeepSound hides data
//! in the low bits of 16-bit PCM samples:
//!   • It scans the audio for a 104-byte header (always "NormalQuality" packed →
//!     26 bytes): `"DSC2"`/`"DSCF"` + quality mode (2/4/8) + AES flag + 20-byte
//!     key hash.
//!   • Then a chain of per-file records: 32-byte head (`"DSSF"` + 20-byte name +
//!     4-byte big-endian size) followed by the file content, each padded to 16
//!     bytes and terminated by a `"DSSF"` marker.
//!   • When encrypted, records are AES-256-**ECB** (no padding). The key is
//!     `SHA-256(UTF-16LE password)` for DSC2, or the ASCII password copied into a
//!     32-byte buffer for the older DSCF.
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::Aes256;
use sha2::{Digest, Sha256};

use super::image_util::input_bytes;
use super::prelude::*;

/// DeepSound only searches this many carrier (DATA) bytes for the header.
const HEAD_SCAN_LIMIT: usize = 352800;

struct Extracted {
    name: String,
    data: Vec<u8>,
}

struct Analysis {
    version: String,
    mode: usize,
    encrypted: bool,
    files: Vec<Extracted>,
}

/// Locate the WAV `data` chunk, returning (payload offset, payload length).
fn find_data_chunk(wav: &[u8]) -> Option<(usize, usize)> {
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return None;
    }
    let mut o = 12usize;
    while o + 8 <= wav.len() {
        let id = &wav[o..o + 4];
        let sz = u32::from_le_bytes([wav[o + 4], wav[o + 5], wav[o + 6], wav[o + 7]]) as usize;
        if id.eq_ignore_ascii_case(b"data") {
            let start = o + 8;
            let len = sz.min(wav.len().saturating_sub(start));
            return Some((start, len));
        }
        o = o + 8 + sz + (sz & 1); // chunks are word-aligned
    }
    None
}

/// Recover `carrier_len / mode` secret bytes from the DATA region at `base`.
/// Caller guarantees `base + carrier_len <= data.len()`.
fn decode_data(data: &[u8], base: usize, carrier_len: usize, mode: usize) -> Vec<u8> {
    let num = carrier_len / mode;
    let mut out = vec![0u8; num];
    match mode {
        2 => {
            for (j, o) in out.iter_mut().enumerate() {
                *o = data[base + j * 2];
            }
        }
        4 => {
            for (k, o) in out.iter_mut().enumerate() {
                let b = base + k * 4;
                *o = ((data[b] & 0xF) << 4) | (data[b + 2] & 0xF);
            }
        }
        8 => {
            for (i, o) in out.iter_mut().enumerate() {
                let b = base + i * 8;
                *o = ((data[b] & 3) << 6)
                    | ((data[b + 2] & 3) << 4)
                    | ((data[b + 4] & 3) << 2)
                    | (data[b + 6] & 3);
            }
        }
        _ => {}
    }
    out
}

/// Scan for the DeepSound header, returning (DATA offset, version string).
fn locate_head(data: &[u8], base: usize, len: usize) -> Option<(usize, String)> {
    let scan = len.min(HEAD_SCAN_LIMIT);
    let mut i = 0;
    while i < scan {
        if i + 104 > len {
            break;
        }
        let dec = decode_data(data, base + i, 104, 4);
        let ver = &dec[0..4];
        if (ver == b"DSC2" || ver == b"DSCF") && matches!(dec[4], 2 | 4 | 8) && matches!(dec[5], 0 | 1)
        {
            return Some((i, String::from_utf8_lossy(ver).into_owned()));
        }
        i += 1;
    }
    None
}

fn key_dsc2(pw: &str) -> [u8; 32] {
    let utf16: Vec<u8> = pw.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    Sha256::digest(utf16).into()
}

fn key_dscf(pw: &str) -> [u8; 32] {
    let mut k = [0u8; 32];
    let bytes: Vec<u8> = pw
        .chars()
        .map(|c| if (c as u32) < 128 { c as u8 } else { b'?' })
        .collect();
    let n = bytes.len().min(32);
    k[..n].copy_from_slice(&bytes[..n]);
    k
}

fn aes_ecb_decrypt(key: &[u8; 32], buf: &mut [u8]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    for chunk in buf.chunks_exact_mut(16) {
        cipher.decrypt_block(GenericArray::from_mut_slice(chunk));
    }
}

fn extract(wav: &[u8], password: &str) -> Result<Analysis, CoreError> {
    let (base, len) = find_data_chunk(wav).ok_or_else(|| {
        CoreError::Parse("不是有效的 WAV（缺少 data 块）。DeepSound 只支持 PCM WAV。".into())
    })?;
    let (h, version) = locate_head(wav, base, len).ok_or_else(|| {
        CoreError::Other(
            "未找到 DeepSound 头（DSC2/DSCF）。可能不是 DeepSound 文件，或不是 PCM WAV。".into(),
        )
    })?;

    let head = decode_data(wav, base + h, 104, 4);
    let mode = head[4] as usize;
    let encrypted = head[5] == 1;

    let key = if encrypted {
        if password.is_empty() {
            return Err(CoreError::Other(format!(
                "DeepSound 文件已加密（{version}），请在参数里填入密码。"
            )));
        }
        Some(if version == "DSCF" {
            key_dscf(password)
        } else {
            key_dsc2(password)
        })
    } else {
        None
    };

    // Encrypted heads occupy the full 104 carrier bytes; plain heads only 24.
    let mut pos = if encrypted { h + 104 } else { h + 24 };
    let mut files = Vec::new();
    loop {
        let hdr_carrier = 32 * mode;
        if base + pos + hdr_carrier > base + len {
            break;
        }
        let mut hdr = decode_data(wav, base + pos, hdr_carrier, mode);
        if let Some(k) = &key {
            aes_ecb_decrypt(k, &mut hdr);
        }
        if &hdr[0..4] != b"DSSF" {
            if encrypted && files.is_empty() {
                return Err(CoreError::Other("密码错误（解密后未出现 DSSF 记录）。".into()));
            }
            break;
        }
        let name = String::from_utf8_lossy(&hdr[4..24])
            .trim_matches('\0')
            .replace('?', "X");
        let size = u32::from_be_bytes([hdr[24], hdr[25], hdr[26], hdr[27]]) as usize;

        let cstart = pos + hdr_carrier;
        let pad_len = 16 - (size + 4) % 16;
        let padded = size + pad_len + 4; // content + zero pad + "DSSF" marker
        let content_carrier = padded * mode;
        if base + cstart + content_carrier > base + len {
            break; // truncated / bad size
        }
        let mut content = decode_data(wav, base + cstart, content_carrier, mode);
        if let Some(k) = &key {
            aes_ecb_decrypt(k, &mut content);
        }
        content.truncate(size);
        files.push(Extracted { name, data: content });
        pos = cstart + content_carrier;
    }

    Ok(Analysis {
        version,
        mode,
        encrypted,
        files,
    })
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let bytes = input_bytes(inputs, "data")?;
        let password = pstr(p, "password", "");
        let a = extract(&bytes, password)?;

        let mut report = format!(
            "DeepSound {} · 质量模式 {} · {}\n共 {} 个隐藏文件：",
            a.version,
            a.mode,
            if a.encrypted { "AES-256 加密" } else { "未加密" },
            a.files.len()
        );
        for f in &a.files {
            report.push_str(&format!("\n - {} ({} 字节)", f.name, f.data.len()));
        }

        let mut m = PortMap::new();
        match a.files.first() {
            Some(first) => {
                m.insert(
                    "bytes".into(),
                    PortValue::Bytes(Arc::from(first.data.clone().into_boxed_slice())),
                );
                m.insert(
                    "text".into(),
                    PortValue::Text(String::from_utf8_lossy(&first.data).into_owned()),
                );
                m.insert("filename".into(), PortValue::Text(first.name.clone()));
            }
            None => {
                m.insert("bytes".into(), PortValue::Bytes(Arc::from(Vec::new().into_boxed_slice())));
                m.insert("text".into(), PortValue::Text(String::new()));
                m.insert("filename".into(), PortValue::Text(String::new()));
            }
        }
        m.insert("report".into(), PortValue::Text(report));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "deepsound_extract",
            AUD,
            "DeepSound 提取",
            FUCHSIA,
            vec![req("data", "音频", PortType::Any)],
            vec![
                req("bytes", "首个文件", PortType::Bytes),
                opt("text", "文本", PortType::Text),
                opt("filename", "文件名", PortType::Text),
                opt("report", "分析", PortType::Text),
            ],
            vec![ParamSpec::text("password", "密码(加密时填)", "", false)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_plain() {
        let wav = include_bytes!("../../tests/fixtures/deepsound_plain.wav");
        let a = extract(wav, "").unwrap();
        assert_eq!(a.version, "DSC2");
        assert!(!a.encrypted);
        assert_eq!(a.files.len(), 1);
        assert_eq!(a.files[0].name, "mini_secret.txt");
        assert_eq!(a.files[0].data, b"flagOK");
    }

    #[test]
    fn extracts_aes() {
        let wav = include_bytes!("../../tests/fixtures/deepsound_aes.wav");
        let a = extract(wav, "pw123").unwrap();
        assert!(a.encrypted);
        assert_eq!(a.files[0].data, b"flagOK");
    }

    #[test]
    fn wrong_password_is_rejected() {
        let wav = include_bytes!("../../tests/fixtures/deepsound_aes.wav");
        assert!(extract(wav, "not-the-password").is_err());
    }

    #[test]
    fn missing_password_is_reported() {
        let wav = include_bytes!("../../tests/fixtures/deepsound_aes.wav");
        assert!(extract(wav, "").is_err());
    }
}
