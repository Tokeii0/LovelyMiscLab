//! Zero-width & invisible-character steganography — hide/reveal data carried by
//! invisible Unicode code points. A staple of CTF misc challenges and real-world
//! "invisible text" tools. Several encoding schemes are supported:
//!   • 二进制      — 1 bit per zero-width char (classic ZWSP/ZWNJ pair)
//!   • 四进制      — 2 bits per char over a 4-symbol alphabet (zwsp-steg style)
//!   • 变体选择符  — 1 byte per Unicode Variation Selector (emoji "smuggling",
//!                   après Paul Butler); handles arbitrary bytes
//!   • Unicode标签 — bytes carried in the Tags block U+E0000‥ (ASCII smuggling /
//!                   invisible prompt-injection technique)
//! Decode's 自动 mode tries every scheme + mapping and returns the best-scoring
//! text, so a challenge string can just be pasted in without knowing the recipe.
use std::collections::HashSet;

use super::prelude::*;

// ---- "digit" characters used by the base-N (二进制 / 四进制) schemes ----
const ZWSP: char = '\u{200B}'; // ZERO WIDTH SPACE
const ZWNJ: char = '\u{200C}'; // ZERO WIDTH NON-JOINER
const ZWJ: char = '\u{200D}'; // ZERO WIDTH JOINER
const ZWNBSP: char = '\u{FEFF}'; // ZERO WIDTH NO-BREAK SPACE (BOM)
const WJ: char = '\u{2060}'; // WORD JOINER
const LRM: char = '\u{200E}'; // LEFT-TO-RIGHT MARK
const RLM: char = '\u{200F}'; // RIGHT-TO-LEFT MARK

/// Code points treated as "zero width" digits when scanning a carrier string
/// (drives frequency analysis + the 二进制/四进制 schemes).
const ZW_SET: &[char] = &[
    ZWSP, ZWNJ, ZWJ, ZWNBSP, WJ, LRM, RLM, '\u{2061}', // FUNCTION APPLICATION
    '\u{2062}', // INVISIBLE TIMES
    '\u{2063}', // INVISIBLE SEPARATOR
    '\u{2064}', // INVISIBLE PLUS
    '\u{061C}', // ARABIC LETTER MARK
    '\u{180E}', // MONGOLIAN VOWEL SEPARATOR
    '\u{034F}', // COMBINING GRAPHEME JOINER
];

/// Default 4-symbol alphabet for the 四进制 (2-bit) scheme (zwsp-steg order).
const BASE4: [char; 4] = [ZWSP, ZWNJ, ZWJ, ZWNBSP];

/// Symbols offered in the 0/1 dropdowns (label ↔ char).
const CHOICES: &[&str] = &[
    "ZWSP (U+200B)",
    "ZWNJ (U+200C)",
    "ZWJ (U+200D)",
    "ZWNBSP (U+FEFF)",
    "WJ (U+2060)",
    "LRM (U+200E)",
    "RLM (U+200F)",
    "INVISIBLE-TIMES (U+2062)",
    "ALM (U+061C)",
    "MVS (U+180E)",
];

/// Scheme labels shared by the encode select (decode also offers 自动 in front).
/// Kept as stable strings because saved flows reference them by value.
const ENCODE_SCHEMES: &[&str] = &["二进制", "四进制", "变体选择符", "Unicode标签"];

fn label_to_char(label: &str) -> char {
    match label {
        "ZWNJ (U+200C)" => ZWNJ,
        "ZWJ (U+200D)" => ZWJ,
        "ZWNBSP (U+FEFF)" => ZWNBSP,
        "WJ (U+2060)" => WJ,
        "LRM (U+200E)" => LRM,
        "RLM (U+200F)" => RLM,
        "INVISIBLE-TIMES (U+2062)" => '\u{2062}',
        "ALM (U+061C)" => '\u{061C}',
        "MVS (U+180E)" => '\u{180E}',
        _ => ZWSP,
    }
}

fn char_name(c: char) -> &'static str {
    match c {
        ZWSP => "ZWSP",
        ZWNJ => "ZWNJ",
        ZWJ => "ZWJ",
        ZWNBSP => "ZWNBSP",
        WJ => "WJ",
        LRM => "LRM",
        RLM => "RLM",
        '\u{2061}' => "FA",
        '\u{2062}' => "INVISIBLE-TIMES",
        '\u{2063}' => "INVISIBLE-SEP",
        '\u{2064}' => "INVISIBLE-PLUS",
        '\u{061C}' => "ALM",
        '\u{180E}' => "MVS",
        '\u{034F}' => "CGJ",
        _ => "?",
    }
}

fn order_name(msb: bool) -> &'static str {
    if msb {
        "MSB"
    } else {
        "LSB"
    }
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// ------------------------------------------------------------ bit helpers

fn bytes_to_bits(bytes: &[u8], msb: bool) -> String {
    let mut s = String::with_capacity(bytes.len() * 8);
    for &b in bytes {
        for i in 0..8 {
            let shift = if msb { 7 - i } else { i };
            s.push(if (b >> shift) & 1 == 1 { '1' } else { '0' });
        }
    }
    s
}

fn bits_to_bytes(bits: &str, msb: bool) -> Vec<u8> {
    let flags: Vec<u8> = bits.bytes().filter(|&c| c == b'0' || c == b'1').collect();
    let mut out = Vec::with_capacity(flags.len() / 8);
    for chunk in flags.chunks(8) {
        if chunk.len() < 8 {
            break; // ignore a trailing partial byte
        }
        let mut byte = 0u8;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'1' {
                byte |= 1 << if msb { 7 - i } else { i };
            }
        }
        out.push(byte);
    }
    out
}

// ------------------------------------------------------- scheme: 二进制

fn encode_binary(bytes: &[u8], zero_c: char, one_c: char, msb: bool) -> String {
    bytes_to_bits(bytes, msb)
        .chars()
        .map(|b| if b == '1' { one_c } else { zero_c })
        .collect()
}

fn extract_bits(zw: &[char], zero_c: char, one_c: char) -> String {
    zw.iter()
        .filter_map(|&c| {
            if c == zero_c {
                Some('0')
            } else if c == one_c {
                Some('1')
            } else {
                None
            }
        })
        .collect()
}

// ------------------------------------------------------- scheme: 四进制

/// Pack 2 bits per symbol: pair 00→alphabet[0] … 11→alphabet[3] (zwsp-steg).
fn encode_base4(bytes: &[u8], alphabet: [char; 4], msb: bool) -> String {
    let bits: Vec<u8> = bytes_to_bits(bytes, msb).into_bytes();
    let mut s = String::with_capacity(bits.len() / 2);
    for pair in bits.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        let hi = if pair[0] == b'1' { 2 } else { 0 };
        let lo = if pair[1] == b'1' { 1 } else { 0 };
        s.push(alphabet[hi + lo]);
    }
    s
}

fn decode_base4(text: &str, alphabet: [char; 4], msb: bool) -> (Vec<u8>, String) {
    let mut bits = String::new();
    for c in text.chars() {
        if let Some(idx) = alphabet.iter().position(|&a| a == c) {
            bits.push(if idx & 2 != 0 { '1' } else { '0' });
            bits.push(if idx & 1 != 0 { '1' } else { '0' });
        }
    }
    (bits_to_bytes(&bits, msb), bits)
}

// --------------------------------------------------- scheme: 变体选择符
// Each byte ↔ one Unicode Variation Selector: 0‥15 → VS1‥16 (U+FE00‥FE0F),
// 16‥255 → VS17‥256 (U+E0100‥E01EF). Fully reversible for arbitrary bytes.

fn byte_to_vs(b: u8) -> char {
    let cp = if b < 16 {
        0xFE00 + b as u32
    } else {
        0xE0100 + (b as u32 - 16)
    };
    char::from_u32(cp).expect("variation selector is a valid scalar value")
}

fn vs_to_byte(c: char) -> Option<u8> {
    let cp = c as u32;
    if (0xFE00..=0xFE0F).contains(&cp) {
        Some((cp - 0xFE00) as u8)
    } else if (0xE0100..=0xE01EF).contains(&cp) {
        Some((cp - 0xE0100 + 16) as u8)
    } else {
        None
    }
}

fn encode_vs(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| byte_to_vs(b)).collect()
}

fn decode_vs(text: &str) -> Vec<u8> {
    text.chars().filter_map(vs_to_byte).collect()
}

// --------------------------------------------------- scheme: Unicode标签
// Each byte ↔ one code point in the Tags block: byte b → U+E0000 + b. ASCII
// stays inside the canonical E0000‥E007F tag range; the "ASCII smuggling" and
// invisible prompt-injection payloads live here.

fn byte_to_tag(b: u8) -> char {
    char::from_u32(0xE0000 + b as u32).expect("tag code point is a valid scalar value")
}

fn tag_to_byte(c: char) -> Option<u8> {
    let cp = c as u32;
    if (0xE0000..=0xE00FF).contains(&cp) {
        Some((cp - 0xE0000) as u8)
    } else {
        None
    }
}

fn encode_tags(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| byte_to_tag(b)).collect()
}

fn decode_tags(text: &str) -> Vec<u8> {
    text.chars().filter_map(tag_to_byte).collect()
}

// ---------------------------------------------------------------- shared

/// Insert the hidden run into the cover text at the requested position.
fn weave(cover: &str, hidden: &str, position: &str) -> String {
    if cover.is_empty() {
        return hidden.to_string();
    }
    match position {
        "开头" => format!("{hidden}{cover}"),
        "中间" => {
            let mid = cover.chars().count() / 2;
            let mut s = String::new();
            for (i, ch) in cover.chars().enumerate() {
                if i == mid {
                    s.push_str(hidden);
                }
                s.push(ch);
            }
            s
        }
        _ => format!("{cover}{hidden}"),
    }
}

/// Score a decode candidate: readability + flag-shape bonus − replacement penalty.
fn score(s: &str) -> f32 {
    if s.is_empty() {
        return -10.0;
    }
    let mut sc = english_score(s);
    if s.contains('{') && s.contains('}') {
        sc += 1.0;
    }
    let bad = s.chars().filter(|&c| c == '\u{FFFD}').count();
    sc - bad as f32 * 3.0
}

fn add_cand(cands: &mut Vec<(char, char)>, a: char, b: char) {
    if a != b && !cands.contains(&(a, b)) {
        cands.push((a, b));
    }
}

fn out3(text: &str, bits: &str, report: &str) -> PortMap {
    let mut m = PortMap::new();
    m.insert("text".into(), PortValue::Text(text.to_string()));
    m.insert("bits".into(), PortValue::Text(bits.to_string()));
    m.insert("report".into(), PortValue::Text(report.to_string()));
    m
}

/// One auto-decode attempt, kept for scoring/ranking.
struct Cand {
    text: String,
    bits: String,
    scheme: String,
    sc: f32,
}

fn push_cand(cands: &mut Vec<Cand>, bytes: Vec<u8>, bits: String, scheme: String) {
    let text = lossy(&bytes);
    let sc = score(&text);
    cands.push(Cand {
        text,
        bits,
        scheme,
        sc,
    });
}

// ---------------------------------------------------------------- decode

struct Decode;
impl Node for Decode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?;
        let msb = pbool(params, "msb", true);

        match pstr(params, "scheme", "自动") {
            "二进制" => {
                let zw: Vec<char> = input.chars().filter(|c| ZW_SET.contains(c)).collect();
                let zero_c = label_to_char(pstr(params, "zero", "ZWSP (U+200B)"));
                let one_c = label_to_char(pstr(params, "one", "ZWNJ (U+200C)"));
                let bits = extract_bits(&zw, zero_c, one_c);
                let bytes = bits_to_bytes(&bits, msb);
                let report = format!(
                    "二进制：0={} 1={}（{}），共 {} 位 → {} 字节。",
                    char_name(zero_c),
                    char_name(one_c),
                    order_name(msb),
                    bits.len(),
                    bytes.len()
                );
                Ok(out3(&lossy(&bytes), &bits, &report))
            }
            "四进制" => {
                let (bytes, bits) = decode_base4(input, BASE4, msb);
                let report = format!(
                    "四进制（2位/字符，{}）：共 {} 位 → {} 字节。",
                    order_name(msb),
                    bits.len(),
                    bytes.len()
                );
                Ok(out3(&lossy(&bytes), &bits, &report))
            }
            "变体选择符" => {
                let bytes = decode_vs(input);
                let bits = bytes_to_bits(&bytes, msb);
                let report = format!("变体选择符（1字节/字符）：解出 {} 字节。", bytes.len());
                Ok(out3(&lossy(&bytes), &bits, &report))
            }
            "Unicode标签" => {
                let bytes = decode_tags(input);
                let bits = bytes_to_bits(&bytes, msb);
                let report = format!("Unicode 标签（U+E0000 块）：解出 {} 字节。", bytes.len());
                Ok(out3(&lossy(&bytes), &bits, &report))
            }
            _ => auto_decode(input),
        }
    }
}

/// 自动: try every scheme + mapping, return the best-scoring plaintext.
fn auto_decode(input: &str) -> Result<PortMap, CoreError> {
    let zw: Vec<char> = input.chars().filter(|c| ZW_SET.contains(c)).collect();
    let vs_count = input.chars().filter(|&c| vs_to_byte(c).is_some()).count();
    let tag_count = input.chars().filter(|&c| tag_to_byte(c).is_some()).count();

    if zw.is_empty() && vs_count == 0 && tag_count == 0 {
        return Ok(out3("", "", "未发现零宽/隐形字符。"));
    }

    // Frequency table of zero-width digits in first-seen order.
    let mut counts: Vec<(char, usize)> = Vec::new();
    for &c in &zw {
        match counts.iter_mut().find(|(x, _)| *x == c) {
            Some(e) => e.1 += 1,
            None => counts.push((c, 1)),
        }
    }

    let mut cands: Vec<Cand> = Vec::new();

    // Byte-oriented schemes (no ambiguity to brute-force). Require ≥2 variation
    // selectors so a lone emoji VS16 (U+FE0F) doesn't masquerade as a payload.
    if vs_count >= 2 {
        let bytes = decode_vs(input);
        let bits = bytes_to_bits(&bytes, true);
        push_cand(&mut cands, bytes, bits, "变体选择符".into());
    }
    if tag_count >= 1 {
        let bytes = decode_tags(input);
        let bits = bytes_to_bits(&bytes, true);
        push_cand(&mut cands, bytes, bits, "Unicode标签".into());
    }

    // Base-N digit schemes need ≥2 distinct symbols to carry bits.
    if counts.len() >= 2 {
        let present: HashSet<char> = counts.iter().map(|(c, _)| *c).collect();
        let mut by_freq = counts.clone();
        by_freq.sort_by(|a, b| b.1.cmp(&a.1));

        // --- 二进制: frequency-ranked + canonical pairs, both bit orders ---
        let mut pairs: Vec<(char, char)> = Vec::new();
        add_cand(&mut pairs, by_freq[0].0, by_freq[1].0);
        add_cand(&mut pairs, by_freq[1].0, by_freq[0].0);
        for &(x, y) in &[(ZWSP, ZWNJ), (ZWNJ, ZWJ), (ZWSP, ZWJ)] {
            if present.contains(&x) && present.contains(&y) {
                add_cand(&mut pairs, x, y);
                add_cand(&mut pairs, y, x);
            }
        }
        for &(zero_c, one_c) in &pairs {
            for &order in &[true, false] {
                let bits = extract_bits(&zw, zero_c, one_c);
                let bytes = bits_to_bytes(&bits, order);
                push_cand(
                    &mut cands,
                    bytes,
                    bits,
                    format!(
                        "二进制 0={} 1={} {}",
                        char_name(zero_c),
                        char_name(one_c),
                        order_name(order)
                    ),
                );
            }
        }

        // --- 四进制: default alphabet + a frequency-ranked one, both orders ---
        let mut alphabets: Vec<([char; 4], &str)> = vec![(BASE4, "默认序")];
        let mut ranked = BASE4;
        for (i, slot) in ranked.iter_mut().enumerate() {
            if let Some((c, _)) = by_freq.get(i) {
                *slot = *c;
            }
        }
        if ranked != BASE4 {
            alphabets.push((ranked, "频率序"));
        }
        for (alpha, tag) in &alphabets {
            for &order in &[true, false] {
                let (bytes, bits) = decode_base4(input, *alpha, order);
                if bits.is_empty() {
                    continue;
                }
                push_cand(
                    &mut cands,
                    bytes,
                    bits,
                    format!("四进制 {} {}", tag, order_name(order)),
                );
            }
        }
    }

    // Detection summary shown regardless of outcome.
    let mut parts: Vec<String> = counts
        .iter()
        .map(|(c, n)| format!("{}×{}", char_name(*c), n))
        .collect();
    if vs_count > 0 {
        parts.push(format!("变体选择符×{vs_count}"));
    }
    if tag_count > 0 {
        parts.push(format!("标签×{tag_count}"));
    }
    let found = parts.join(", ");

    if cands.is_empty() {
        let note = if counts.len() == 1 {
            format!(
                "发现零宽字符：{found}。只有 1 种符号，无法二值解码（请切到具体方案并指定映射）。"
            )
        } else {
            format!("发现隐形字符：{found}，但未能自动解码（可尝试指定具体方案）。")
        };
        return Ok(out3("", "", &note));
    }

    cands.sort_by(|a, b| b.sc.partial_cmp(&a.sc).unwrap_or(std::cmp::Ordering::Equal));
    let best = &cands[0];
    let report = format!(
        "发现隐形字符：{found}。自动选用【{}】（评分 {:.2}）。",
        best.scheme, best.sc
    );
    Ok(out3(&best.text, &best.bits, &report))
}

// ---------------------------------------------------------------- encode

struct Encode;
impl Node for Encode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let secret = in_text(inputs, "text")?;
        let msb = pbool(params, "msb", true);
        let bytes = secret.as_bytes();

        let hidden = match pstr(params, "scheme", "二进制") {
            "四进制" => encode_base4(bytes, BASE4, msb),
            "变体选择符" => encode_vs(bytes),
            "Unicode标签" => encode_tags(bytes),
            _ => {
                let zero_c = label_to_char(pstr(params, "zero", "ZWSP (U+200B)"));
                let one_c = label_to_char(pstr(params, "one", "ZWNJ (U+200C)"));
                encode_binary(bytes, zero_c, one_c, msb)
            }
        };

        let cover = pstr(params, "cover", "");
        let result = weave(cover, &hidden, pstr(params, "position", "结尾"));

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(result));
        m.insert("bits".into(), PortValue::Text(bytes_to_bits(bytes, msb)));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let mut decode_schemes = vec!["自动"];
    decode_schemes.extend_from_slice(ENCODE_SCHEMES);
    reg.register(
        desc(
            "zero_width_decode",
            STEG,
            "零宽解码",
            INDIGO,
            vec![req("text", "载体文本", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("bits", "位串", PortType::Text),
                opt("report", "分析", PortType::Text),
            ],
            vec![
                ParamSpec::select("scheme", "模式", &decode_schemes, "自动"),
                ParamSpec::select("zero", "0 = 字符 (二进制)", CHOICES, "ZWSP (U+200B)"),
                ParamSpec::select("one", "1 = 字符 (二进制)", CHOICES, "ZWNJ (U+200C)"),
                ParamSpec::toggle("msb", "高位在前 (MSB)", true),
            ],
        ),
        Arc::new(|| Arc::new(Decode)),
    );
    reg.register(
        desc(
            "zero_width_encode",
            STEG,
            "零宽编码",
            INDIGO,
            vec![req("text", "秘密信息", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("bits", "位串", PortType::Text),
            ],
            vec![
                ParamSpec::text("cover", "载体文本", "The quick brown fox", false),
                ParamSpec::select("scheme", "方案", ENCODE_SCHEMES, "二进制"),
                ParamSpec::select("zero", "0 = 字符 (二进制)", CHOICES, "ZWSP (U+200B)"),
                ParamSpec::select("one", "1 = 字符 (二进制)", CHOICES, "ZWNJ (U+200C)"),
                ParamSpec::select("position", "隐藏位置", &["结尾", "开头", "中间"], "结尾"),
                ParamSpec::toggle("msb", "高位在前 (MSB)", true),
            ],
        ),
        Arc::new(|| Arc::new(Encode)),
    );
}
