//! Zero-width character steganography — hide/reveal data encoded in invisible
//! Unicode code points (ZWSP / ZWNJ / ZWJ / …). A staple of CTF misc challenges:
//! a message looks like ordinary text but carries a bitstream in the gaps.
use std::collections::HashSet;

use super::prelude::*;

/// Code points treated as "zero width" when scanning a carrier string.
const ZW_SET: &[char] = &[
    '\u{200B}', // ZERO WIDTH SPACE
    '\u{200C}', // ZERO WIDTH NON-JOINER
    '\u{200D}', // ZERO WIDTH JOINER
    '\u{FEFF}', // ZERO WIDTH NO-BREAK SPACE (BOM)
    '\u{2060}', // WORD JOINER
    '\u{200E}', // LEFT-TO-RIGHT MARK
    '\u{200F}', // RIGHT-TO-LEFT MARK
];

/// Symbols offered in the 0/1 dropdowns (label ↔ char).
const CHOICES: &[&str] = &[
    "ZWSP (U+200B)",
    "ZWNJ (U+200C)",
    "ZWJ (U+200D)",
    "ZWNBSP (U+FEFF)",
    "WJ (U+2060)",
];

fn label_to_char(label: &str) -> char {
    match label {
        "ZWNJ (U+200C)" => '\u{200C}',
        "ZWJ (U+200D)" => '\u{200D}',
        "ZWNBSP (U+FEFF)" => '\u{FEFF}',
        "WJ (U+2060)" => '\u{2060}',
        _ => '\u{200B}',
    }
}

fn char_name(c: char) -> &'static str {
    match c {
        '\u{200B}' => "ZWSP",
        '\u{200C}' => "ZWNJ",
        '\u{200D}' => "ZWJ",
        '\u{FEFF}' => "ZWNBSP",
        '\u{2060}' => "WJ",
        '\u{200E}' => "LRM",
        '\u{200F}' => "RLM",
        _ => "?",
    }
}

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
        let zw: Vec<char> = input.chars().filter(|c| ZW_SET.contains(c)).collect();

        if zw.is_empty() {
            return Ok(out3("", "", "未发现零宽字符。"));
        }

        // Frequency table in first-seen order.
        let mut counts: Vec<(char, usize)> = Vec::new();
        for &c in &zw {
            match counts.iter_mut().find(|(x, _)| *x == c) {
                Some(e) => e.1 += 1,
                None => counts.push((c, 1)),
            }
        }
        let found = counts
            .iter()
            .map(|(c, n)| format!("{}×{}", char_name(*c), n))
            .collect::<Vec<_>>()
            .join(", ");

        let msb = pbool(params, "msb", true);

        if pstr(params, "scheme", "自动") == "二进制" {
            let zero_c = label_to_char(pstr(params, "zero", "ZWSP (U+200B)"));
            let one_c = label_to_char(pstr(params, "one", "ZWNJ (U+200C)"));
            let bits = extract_bits(&zw, zero_c, one_c);
            let text = String::from_utf8_lossy(&bits_to_bytes(&bits, msb)).into_owned();
            let report = format!(
                "发现零宽字符：{found}。映射 0={} 1={}（{}），共 {} 位。",
                char_name(zero_c),
                char_name(one_c),
                if msb { "MSB" } else { "LSB" },
                bits.len()
            );
            return Ok(out3(&text, &bits, &report));
        }

        // 自动: try frequency-ranked and canonical pairs, both bit orders.
        if counts.len() < 2 {
            let report = format!(
                "发现零宽字符：{found}。只有 1 种符号，无法二值解码（请切到「二进制」并指定映射）。"
            );
            return Ok(out3("", "", &report));
        }

        let present: HashSet<char> = counts.iter().map(|(c, _)| *c).collect();
        let mut by_freq = counts.clone();
        by_freq.sort_by(|a, b| b.1.cmp(&a.1));

        let mut cands: Vec<(char, char)> = Vec::new();
        add_cand(&mut cands, by_freq[0].0, by_freq[1].0);
        add_cand(&mut cands, by_freq[1].0, by_freq[0].0);
        for &(x, y) in &[
            ('\u{200B}', '\u{200C}'),
            ('\u{200C}', '\u{200D}'),
            ('\u{200B}', '\u{200D}'),
        ] {
            if present.contains(&x) && present.contains(&y) {
                add_cand(&mut cands, x, y);
                add_cand(&mut cands, y, x);
            }
        }

        let mut best: Option<(String, char, char, bool)> = None;
        let mut best_score = f32::MIN;
        for &(zero_c, one_c) in &cands {
            for &order in &[true, false] {
                let text =
                    String::from_utf8_lossy(&bits_to_bytes(&extract_bits(&zw, zero_c, one_c), order))
                        .into_owned();
                let sc = score(&text);
                if sc > best_score {
                    best_score = sc;
                    best = Some((text, zero_c, one_c, order));
                }
            }
        }

        let (text, zero_c, one_c, order) = best.expect("cands non-empty when >=2 symbols");
        let bits = extract_bits(&zw, zero_c, one_c);
        let report = format!(
            "发现零宽字符：{found}。自动选用 0={} 1={}（{}），共 {} 位。",
            char_name(zero_c),
            char_name(one_c),
            if order { "MSB" } else { "LSB" },
            bits.len()
        );
        Ok(out3(&text, &bits, &report))
    }
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
        let cover = pstr(params, "cover", "");
        let zero_c = label_to_char(pstr(params, "zero", "ZWSP (U+200B)"));
        let one_c = label_to_char(pstr(params, "one", "ZWNJ (U+200C)"));
        let msb = pbool(params, "msb", true);

        let bits = bytes_to_bits(secret.as_bytes(), msb);
        let hidden: String = bits
            .chars()
            .map(|b| if b == '1' { one_c } else { zero_c })
            .collect();

        let result = if cover.is_empty() {
            hidden.clone()
        } else {
            match pstr(params, "position", "结尾") {
                "开头" => format!("{hidden}{cover}"),
                "中间" => {
                    let mid = cover.chars().count() / 2;
                    let mut s = String::new();
                    for (i, ch) in cover.chars().enumerate() {
                        if i == mid {
                            s.push_str(&hidden);
                        }
                        s.push(ch);
                    }
                    s
                }
                _ => format!("{cover}{hidden}"),
            }
        };

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(result));
        m.insert("bits".into(), PortValue::Text(bits));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
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
                ParamSpec::select("scheme", "模式", &["自动", "二进制"], "自动"),
                ParamSpec::select("zero", "0 = 字符", CHOICES, "ZWSP (U+200B)"),
                ParamSpec::select("one", "1 = 字符", CHOICES, "ZWNJ (U+200C)"),
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
                ParamSpec::select("zero", "0 = 字符", CHOICES, "ZWSP (U+200B)"),
                ParamSpec::select("one", "1 = 字符", CHOICES, "ZWNJ (U+200C)"),
                ParamSpec::select("position", "隐藏位置", &["结尾", "开头", "中间"], "结尾"),
                ParamSpec::toggle("msb", "高位在前 (MSB)", true),
            ],
        ),
        Arc::new(|| Arc::new(Encode)),
    );
}
