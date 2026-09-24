//! StegCloak-compatible steganography — hide an (optionally encrypted + HMAC'd)
//! secret inside ordinary text using six invisible characters, and reveal it.
//! Byte-compatible with the `stegcloak` npm tool for the common case:
//!   secret → [lzutf8 compress] → compliment(~) → [AES-256-CTR + salt/HMAC] →
//!   base-4 over 6 zero-width chars → run-length "shrink" → embed after a word.
//! Key derivation is PBKDF2-HMAC-SHA512 (10000 rounds, 48 bytes → 16 IV + 32 key).
//!
//! NOTE on compression: StegCloak runs the payload through `lzutf8`, but for text
//! without long repeated runs that step is the identity (verified against the
//! real tool), so this port skips it. Secrets with long repeats would need the
//! LZ codec — decode flags that case in its report rather than silently corrupt.
use aes::cipher::{KeyIvInit, StreamCipher};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};

use super::prelude::*;

/// StegCloak's six invisible characters. Index 0..=3 each carry 2 bits (base-4);
/// index 4/5 are run-length markers. Order defines the mapping — do not reorder.
const ZWC: [char; 6] = [
    '\u{200C}', // ZERO WIDTH NON-JOINER
    '\u{200D}', // ZERO WIDTH JOINER
    '\u{2061}', // FUNCTION APPLICATION
    '\u{2062}', // INVISIBLE TIMES
    '\u{2063}', // INVISIBLE SEPARATOR  (RLE marker)
    '\u{2064}', // INVISIBLE PLUS       (RLE marker)
];

/// The six ordered index-pairs used to encode which two symbols were run-length
/// compressed (StegCloak's `tableMap`): a compress-flag `ZWC[i]` ⇒ pair `[i]`.
const TABLE_MAP: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

type Aes256Ctr = ctr::Ctr128BE<aes::Aes256>;
type HmacSha256 = Hmac<Sha256>;

fn zwc_index(c: char) -> Option<usize> {
    ZWC.iter().position(|&z| z == c)
}

// ------------------------------------------------------------ bit helpers

/// Bytes → MSB-first binary string (StegCloak `byteToBin`).
fn byte_to_bin(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 8);
    for &b in bytes {
        for i in (0..8).rev() {
            s.push(if (b >> i) & 1 == 1 { '1' } else { '0' });
        }
    }
    s
}

/// MSB-first binary string → bytes (StegCloak `binToByte`); trailing <8 dropped.
fn bin_to_bytes(bits: &str) -> Vec<u8> {
    let b = bits.as_bytes();
    let mut out = Vec::with_capacity(b.len() / 8);
    for chunk in b.chunks(8) {
        if chunk.len() < 8 {
            break;
        }
        let mut v = 0u8;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'1' {
                v |= 1 << (7 - i);
            }
        }
        out.push(v);
    }
    out
}

/// Byte-wise bitwise NOT (StegCloak `compliment`); its own inverse.
fn compliment(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().map(|b| !b).collect()
}

// ------------------------------------------------------- ZWC (de)serialize

/// bits → crypt-flag + base-4 ZWC stream (StegCloak `dataToZWC`).
fn data_to_zwc(bits: &str, crypt: bool, integrity: bool) -> String {
    let flag = if integrity && crypt {
        ZWC[0]
    } else if crypt {
        ZWC[1]
    } else {
        ZWC[2]
    };
    let mut s = String::with_capacity(bits.len() / 2 + 1);
    s.push(flag);
    let b = bits.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        let hi = if b[i] == b'1' { 2 } else { 0 };
        let lo = if b[i + 1] == b'1' { 1 } else { 0 };
        s.push(ZWC[hi + lo]);
        i += 2;
    }
    s
}

struct Concealed {
    crypt: bool,
    integrity: bool,
    data: Vec<u8>,
}

/// crypt-flag + base-4 ZWC stream → payload bytes (StegCloak `concealToData`).
fn conceal_to_data(expanded: &str) -> Result<Concealed, CoreError> {
    let mut it = expanded.chars();
    let flag = it
        .next()
        .ok_or_else(|| CoreError::Parse("StegCloak 数据为空。".into()))?;
    let (crypt, integrity) = match zwc_index(flag) {
        Some(0) => (true, true),
        Some(1) => (true, false),
        Some(2) => (false, false),
        _ => return Err(CoreError::Parse("StegCloak 加密标志无效。".into())),
    };
    let mut bits = String::new();
    for c in it {
        let idx =
            zwc_index(c).ok_or_else(|| CoreError::Parse("StegCloak 数据流含非法字符。".into()))?;
        bits.push(if idx & 2 != 0 { '1' } else { '0' });
        bits.push(if idx & 1 != 0 { '1' } else { '0' });
    }
    Ok(Concealed {
        crypt,
        integrity,
        data: bin_to_bytes(&bits),
    })
}

// ------------------------------------------------------- shrink / expand

/// Non-overlapping, left-to-right replace of `target``target` → `marker`.
fn replace_pairs(chars: &[char], target: char, marker: char) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == target && i + 1 < chars.len() && chars[i + 1] == target {
            out.push(marker);
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Run-length compress the ZWC stream (StegCloak `shrink`). We always pick the
/// canonical pair (ZWC[0],ZWC[1]) → compress-flag ZWC[0]; any valid choice is
/// reversible by the standard `expand`, so this stays tool-compatible.
fn shrink(stream: &str) -> String {
    let chars: Vec<char> = stream.chars().collect();
    let step1 = replace_pairs(&chars, ZWC[0], ZWC[4]);
    let step2 = replace_pairs(&step1, ZWC[1], ZWC[5]);
    let mut out = String::with_capacity(step2.len() + 1);
    out.push(ZWC[0]); // compress-flag for pair (ZWC[0], ZWC[1]) == TABLE_MAP[0]
    out.extend(step2);
    out
}

/// Reverse `shrink` (StegCloak `expand`): read the compress-flag, restore runs.
fn expand(stream: &str) -> Result<String, CoreError> {
    let mut it = stream.chars();
    let flag = it
        .next()
        .ok_or_else(|| CoreError::Parse("StegCloak 隐藏流为空。".into()))?;
    let fi = zwc_index(flag)
        .filter(|&i| i < TABLE_MAP.len())
        .ok_or_else(|| CoreError::Parse("StegCloak 压缩标志无效。".into()))?;
    let (ai, bi) = TABLE_MAP[fi];
    let (a, b) = (ZWC[ai], ZWC[bi]);
    let mut out = String::new();
    for c in it {
        if c == ZWC[4] {
            out.push(a);
            out.push(a);
        } else if c == ZWC[5] {
            out.push(b);
            out.push(b);
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

// ------------------------------------------------------- embed / detach

/// Insert the invisible stream after the first word of the cover.
fn embed(cover: &str, stream: &str) -> String {
    let words: Vec<&str> = cover.split(' ').collect();
    if words.len() >= 2 {
        let mut out = String::new();
        out.push_str(words[0]);
        out.push(' ');
        out.push_str(stream);
        out.push_str(words[1]);
        for w in &words[2..] {
            out.push(' ');
            out.push_str(w);
        }
        out
    } else {
        // No second word to hide behind — keep the run leading so detach finds it.
        format!("{stream}{cover}")
    }
}

/// Pull out the invisible stream: the longest contiguous run of ZWC characters.
fn detach(text: &str) -> Result<String, CoreError> {
    let chars: Vec<char> = text.chars().collect();
    let (mut best_start, mut best_len) = (0usize, 0usize);
    let mut i = 0;
    while i < chars.len() {
        if zwc_index(chars[i]).is_some() {
            let start = i;
            while i < chars.len() && zwc_index(chars[i]).is_some() {
                i += 1;
            }
            if i - start > best_len {
                best_len = i - start;
                best_start = start;
            }
        } else {
            i += 1;
        }
    }
    if best_len == 0 {
        return Err(CoreError::Parse("未检测到 StegCloak 隐藏流。".into()));
    }
    Ok(chars[best_start..best_start + best_len].iter().collect())
}

// ------------------------------------------------------------- crypto

fn derive_iv_key(password: &str, salt: &[u8]) -> ([u8; 16], [u8; 32]) {
    let mut out = [0u8; 48];
    pbkdf2::pbkdf2_hmac::<Sha512>(password.as_bytes(), salt, 10000, &mut out);
    let mut iv = [0u8; 16];
    let mut key = [0u8; 32];
    iv.copy_from_slice(&out[0..16]);
    key.copy_from_slice(&out[16..48]);
    (iv, key)
}

fn aes_ctr(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let mut buf = data.to_vec();
    let mut c = Aes256Ctr::new(key.into(), iv.into());
    c.apply_keystream(&mut buf);
    buf
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    let out = mac.finalize().into_bytes();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

// ------------------------------------------------------------- reveal

/// Returns (secret, human report).
fn reveal(text: &str, password: &str) -> Result<(String, String), CoreError> {
    let stream = detach(text)?;
    let expanded = expand(&stream)?;
    let Concealed {
        crypt,
        integrity,
        data,
    } = conceal_to_data(&expanded)?;

    let mut report = String::new();
    let plain = if crypt {
        if data.len() < 8 {
            return Err(CoreError::Parse("StegCloak 数据过短（缺少盐）。".into()));
        }
        let (iv, key) = derive_iv_key(password, &data[0..8]);
        let (stored_hmac, ct) = if integrity {
            if data.len() < 40 {
                return Err(CoreError::Parse("StegCloak 数据过短（缺少 HMAC）。".into()));
            }
            (Some(&data[8..40]), &data[40..])
        } else {
            (None, &data[8..])
        };
        let decrypted = aes_ctr(&key, &iv, ct);
        match stored_hmac {
            Some(stored) if hmac_sha256(&key, &decrypted) == stored => {
                report.push_str("已加密 · HMAC 完整性校验通过")
            }
            Some(_) => report.push_str("已加密 · ⚠ HMAC 校验失败（密码错误或数据被篡改）"),
            None => report.push_str("已加密（无完整性校验）"),
        }
        decrypted
    } else {
        report.push_str("未加密");
        data
    };

    // secret = decompress(compliment(plain)); compress/decompress are identity here.
    let compressed = compliment(&plain);
    let (secret, maybe_lz) = match std::str::from_utf8(&compressed) {
        Ok(s) => (s.to_string(), false),
        Err(_) => (String::from_utf8_lossy(&compressed).into_owned(), true),
    };
    if maybe_lz {
        report.push_str("；⚠ 疑似 lzutf8 压缩载荷（含长重复），本工具未实现该解压，结果可能不完整");
    }
    Ok((secret, report))
}

// ------------------------------------------------------------- hide

fn hide(
    secret: &str,
    password: &str,
    cover: &str,
    crypt: bool,
    integrity: bool,
) -> Result<String, CoreError> {
    let plain = compliment(secret.as_bytes()); // compliment(compress(secret)), compress = identity

    let payload = if crypt {
        let mut salt = [0u8; 8];
        getrandom::getrandom(&mut salt)
            .map_err(|e| CoreError::Parse(format!("随机数生成失败: {e}")))?;
        let (iv, key) = derive_iv_key(password, &salt);
        let ct = aes_ctr(&key, &iv, &plain);
        let mut p = Vec::with_capacity(8 + 32 + ct.len());
        p.extend_from_slice(&salt);
        if integrity {
            p.extend_from_slice(&hmac_sha256(&key, &plain));
        }
        p.extend_from_slice(&ct);
        p
    } else {
        plain
    };

    let stream = shrink(&data_to_zwc(&byte_to_bin(&payload), crypt, integrity));
    Ok(embed(cover, &stream))
}

// ------------------------------------------------------------- nodes

struct Hide;
impl Node for Hide {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let secret = in_text(inputs, "text")?;
        let out = hide(
            secret,
            pstr(params, "password", ""),
            pstr(params, "cover", "This is a confidential message"),
            pbool(params, "encrypt", true),
            pbool(params, "integrity", false),
        )?;
        Ok(out_text(out))
    }
}

struct Reveal;
impl Node for Reveal {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let (secret, report) = reveal(in_text(inputs, "text")?, pstr(params, "password", ""))?;
        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(secret));
        m.insert("report".into(), PortValue::Text(report));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "stegcloak_hide",
            STEG,
            "StegCloak 编码",
            FUCHSIA,
            vec![req("text", "秘密信息", PortType::Text)],
            vec![req("text", "结果", PortType::Text)],
            vec![
                ParamSpec::text(
                    "cover",
                    "载体文本(≥2词)",
                    "This is a confidential message",
                    false,
                ),
                ParamSpec::text("password", "密码", "", false),
                ParamSpec::toggle("encrypt", "加密 (AES-256-CTR)", true),
                ParamSpec::toggle("integrity", "HMAC 完整性校验", false),
            ],
        ),
        Arc::new(|| Arc::new(Hide)),
    );
    reg.register(
        desc(
            "stegcloak_reveal",
            STEG,
            "StegCloak 解码",
            FUCHSIA,
            vec![req("text", "载体文本", PortType::Text)],
            vec![
                req("text", "秘密信息", PortType::Text),
                opt("report", "分析", PortType::Text),
            ],
            vec![ParamSpec::text("password", "密码", "", false)],
        ),
        Arc::new(|| Arc::new(Reveal)),
    );
}
