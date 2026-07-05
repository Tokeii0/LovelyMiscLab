//! Classical-cipher analysis helpers: Vigenere auto analysis, repeating-key XOR
//! cracking, and small-key brute force for rail fence / affine / columnar
//! transposition ciphers.

use serde_json::json;

use super::prelude::*;

#[derive(Clone)]
struct CrackCandidate {
    text: String,
    score: f32,
    note: String,
    key: String,
}

fn finish_candidates(mut c: Vec<CrackCandidate>, limit: usize, bytes: Option<Vec<u8>>) -> PortMap {
    c.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    c.truncate(limit.max(1));
    let best = c.first().cloned();
    let text = c
        .iter()
        .enumerate()
        .map(|(i, x)| {
            format!(
                "#{} score={:.3} {} {}\n{}",
                i + 1,
                x.score,
                x.note,
                x.key,
                x.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let candidates = c
        .iter()
        .map(|x| ScoredString {
            text: x.text.clone(),
            score: x.score,
            note: Some(format!("{} {}", x.note, x.key)),
        })
        .collect::<Vec<_>>();
    let json_rows = c
        .iter()
        .map(|x| json!({ "score": x.score, "note": x.note, "key": x.key, "text": x.text }))
        .collect::<Vec<_>>();

    let mut m = PortMap::new();
    m.insert(
        "best".into(),
        PortValue::Text(best.as_ref().map(|x| x.text.clone()).unwrap_or_default()),
    );
    m.insert(
        "key".into(),
        PortValue::Text(best.as_ref().map(|x| x.key.clone()).unwrap_or_default()),
    );
    m.insert("text".into(), PortValue::Text(text));
    m.insert("candidates".into(), PortValue::Candidates(candidates));
    m.insert("json".into(), PortValue::Json(json!(json_rows)));
    if let Some(b) = bytes {
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(b.into_boxed_slice())),
        );
    }
    m
}

fn crack_score(text: &str) -> f32 {
    let mut score = english_score(text);
    let lower = text.to_ascii_lowercase();
    let padded = format!(" {lower} ");
    if lower.contains("flag{") {
        score += 6.0;
    }
    if lower.contains("ctf{") {
        score += 5.0;
    }
    for token in lower.split(|c: char| !c.is_ascii_alphabetic()) {
        score += match token {
            "the" => 0.75,
            "and" | "that" | "this" | "with" | "from" | "have" | "over" => 0.45,
            "is" | "in" | "to" | "of" | "for" | "on" | "as" | "at" => 0.3,
            "quick" | "brown" | "jumps" | "lazy" | "attack" | "cipher" | "secret" | "readable"
            | "english" | "sentence" | "scoring" => 0.65,
            "flag" | "ctf" => 1.5,
            "vigenere" | "classic" | "columnar" | "repeating" => 0.8,
            _ => 0.0,
        };
    }
    for (phrase, weight) in [
        (" the ", 0.35),
        (" and ", 0.25),
        (" flag ", 0.6),
        (" flag{", 2.0),
        ("attack", 0.45),
        ("cipher", 0.35),
        ("english", 0.35),
    ] {
        score += padded.matches(phrase).count() as f32 * weight;
    }
    let mut alpha = 0usize;
    let mut vowels = 0usize;
    for b in lower.bytes().filter(|b| b.is_ascii_alphabetic()) {
        alpha += 1;
        if matches!(b, b'a' | b'e' | b'i' | b'o' | b'u' | b'y') {
            vowels += 1;
        }
    }
    if alpha >= 16 {
        let ratio = vowels as f32 / alpha as f32;
        if (0.30..=0.50).contains(&ratio) {
            score += 0.35;
        } else {
            score -= (ratio - 0.40).abs().min(0.4);
        }
    }
    if text.contains('�') {
        score -= 1.0;
    }
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        score -= 1.0;
    }
    score
}

// ------------------------------------------------------------- Vigenere

const EN_FREQ: [f64; 26] = [
    0.0812, 0.0149, 0.0271, 0.0432, 0.1202, 0.0230, 0.0203, 0.0592, 0.0731, 0.0010, 0.0069, 0.0398,
    0.0261, 0.0695, 0.0768, 0.0182, 0.0011, 0.0602, 0.0628, 0.0910, 0.0288, 0.0111, 0.0209, 0.0017,
    0.0211, 0.0007,
];

fn letters_upper(text: &str) -> Vec<u8> {
    text.bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase() - b'A')
        .collect()
}

fn index_of_coincidence(col: &[u8]) -> f64 {
    if col.len() < 2 {
        return 0.0;
    }
    let mut counts = [0usize; 26];
    for &x in col {
        counts[x as usize] += 1;
    }
    let num: usize = counts.iter().map(|&n| n * n.saturating_sub(1)).sum();
    num as f64 / (col.len() * (col.len() - 1)) as f64
}

fn avg_ic(letters: &[u8], key_len: usize) -> f64 {
    let mut sum = 0.0;
    for k in 0..key_len {
        let col: Vec<u8> = letters.iter().skip(k).step_by(key_len).copied().collect();
        sum += index_of_coincidence(&col);
    }
    sum / key_len as f64
}

fn chi_square_shift(col: &[u8], shift: u8) -> f64 {
    if col.is_empty() {
        return f64::MAX;
    }
    let mut counts = [0usize; 26];
    for &x in col {
        let plain = (26 + x as i16 - shift as i16) as usize % 26;
        counts[plain] += 1;
    }
    let n = col.len() as f64;
    let mut chi = 0.0;
    for i in 0..26 {
        let expected = n * EN_FREQ[i].max(0.0001);
        let diff = counts[i] as f64 - expected;
        chi += diff * diff / expected;
    }
    chi
}

fn top_vigenere_shifts(col: &[u8], n: usize) -> Vec<(u8, f64)> {
    let mut shifts = (0..26)
        .map(|s| (s, chi_square_shift(col, s)))
        .collect::<Vec<_>>();
    shifts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    shifts.truncate(n);
    shifts
}

fn derive_vigenere_keys(letters: &[u8], key_len: usize, beam_width: usize) -> Vec<(Vec<u8>, f64)> {
    let mut beam = vec![(Vec::<u8>::new(), 0.0f64)];
    for k in 0..key_len {
        let col: Vec<u8> = letters.iter().skip(k).step_by(key_len).copied().collect();
        let shifts = top_vigenere_shifts(&col, 8);
        let mut next = Vec::new();
        for (prefix, prefix_chi) in &beam {
            for &(shift, chi) in &shifts {
                let mut key = prefix.clone();
                key.push(shift);
                next.push((key, prefix_chi + chi));
            }
        }
        next.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        next.truncate(beam_width.max(1));
        beam = next;
    }
    beam
}

fn vigenere_decrypt(text: &str, key: &[u8]) -> String {
    let mut ki = 0usize;
    text.chars()
        .map(|c| {
            let base = if c.is_ascii_lowercase() {
                b'a'
            } else if c.is_ascii_uppercase() {
                b'A'
            } else {
                return c;
            };
            let x = c as u8 - base;
            let k = key[ki % key.len()];
            ki += 1;
            (base + (26 + x - k) % 26) as char
        })
        .collect()
}

fn vigenere_key_chi(letters: &[u8], key: &[u8]) -> f64 {
    key.iter()
        .enumerate()
        .map(|(pos, &shift)| {
            let col: Vec<u8> = letters
                .iter()
                .skip(pos)
                .step_by(key.len())
                .copied()
                .collect();
            chi_square_shift(&col, shift)
        })
        .sum()
}

fn refine_vigenere_key(text: &str, seed: &[u8]) -> (Vec<u8>, String, f32) {
    let mut best_key = seed.to_vec();
    let mut best_text = vigenere_decrypt(text, &best_key);
    let mut best_score = crack_score(&best_text);

    for _ in 0..3 {
        let mut changed = false;
        for pos in 0..best_key.len() {
            let original = best_key[pos];
            let mut local_shift = original;
            let mut local_text = best_text.clone();
            let mut local_score = best_score;

            for shift in 0..26 {
                if shift == original {
                    continue;
                }
                let mut trial_key = best_key.clone();
                trial_key[pos] = shift;
                let trial_text = vigenere_decrypt(text, &trial_key);
                let trial_score = crack_score(&trial_text);
                if trial_score > local_score + 0.0001 {
                    local_shift = shift;
                    local_text = trial_text;
                    local_score = trial_score;
                }
            }

            if local_shift != original {
                best_key[pos] = local_shift;
                best_text = local_text;
                best_score = local_score;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    (best_key, best_text, best_score)
}

struct VigenereAnalyze;

impl Node for VigenereAnalyze {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(i, "text")?;
        let letters = letters_upper(input);
        if letters.len() < 8 {
            return Err(CoreError::Parse(
                "文本中字母太少，无法分析维吉尼亚密钥".into(),
            ));
        }
        let max_len = (pnum(p, "maxKeyLen", 16.0) as usize)
            .clamp(1, 64)
            .min(letters.len());
        let top = (pnum(p, "top", 10.0) as usize).clamp(1, 50);

        let mut candidates = Vec::new();
        for len in 1..=max_len {
            let ic = avg_ic(&letters, len);
            let refine_limit = if len <= 16 { 96 } else { 24 };
            for (idx, (key, chi)) in derive_vigenere_keys(&letters, len, 512)
                .into_iter()
                .enumerate()
            {
                let key_text: String = key.iter().map(|&k| (b'A' + k) as char).collect();
                let plain = vigenere_decrypt(input, &key);
                let score = crack_score(&plain) + ((ic - 0.038).max(0.0) * 3.0) as f32
                    - (chi / letters.len() as f64 * 0.02) as f32;
                candidates.push(CrackCandidate {
                    text: plain,
                    score,
                    note: format!("len={len} ic={ic:.4} chi={chi:.1}"),
                    key: format!("key={key_text}"),
                });

                if idx < refine_limit {
                    let (refined_key, refined_plain, refined_plain_score) =
                        refine_vigenere_key(input, &key);
                    if refined_key != key {
                        let refined_chi = vigenere_key_chi(&letters, &refined_key);
                        let refined_key_text: String =
                            refined_key.iter().map(|&k| (b'A' + k) as char).collect();
                        let refined_score = refined_plain_score
                            + ((ic - 0.038).max(0.0) * 3.0) as f32
                            - (refined_chi / letters.len() as f64 * 0.02) as f32;
                        candidates.push(CrackCandidate {
                            text: refined_plain,
                            score: refined_score,
                            note: format!("len={len} ic={ic:.4} chi={refined_chi:.1} refined"),
                            key: format!("key={refined_key_text}"),
                        });
                    }
                }
            }
        }
        Ok(finish_candidates(candidates, top, None))
    }
}

// ------------------------------------------------------------- Repeating XOR

fn decode_input(data: Vec<u8>, format: &str) -> Result<Vec<u8>, CoreError> {
    match format {
        "Hex" => hex::decode(
            String::from_utf8_lossy(&data)
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>(),
        )
        .map_err(|e| CoreError::Parse(format!("Hex 无效: {e}"))),
        "Base64" => {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD
                .decode(String::from_utf8_lossy(&data).trim())
                .map_err(|e| CoreError::Parse(format!("Base64 无效: {e}")))
        }
        _ => Ok(data),
    }
}

fn byte_score(b: u8) -> f32 {
    match b {
        b'a'..=b'z' | b'A'..=b'Z' | b' ' => 2.0,
        b'0'..=b'9' => 1.1,
        b'\n' | b'\r' | b'\t' => 0.4,
        0x21..=0x2f | 0x3a..=0x40 | 0x5b..=0x60 | 0x7b..=0x7e => 0.5,
        0x00..=0x08 | 0x0b | 0x0c | 0x0e..=0x1f | 0x7f => -5.0,
        0x80..=0xff => -2.0,
    }
}

fn xor_key_for_len(data: &[u8], key_len: usize) -> Vec<u8> {
    (0..key_len)
        .map(|pos| {
            (0u8..=255)
                .max_by(|&a, &b| {
                    let sa: f32 = data
                        .iter()
                        .skip(pos)
                        .step_by(key_len)
                        .map(|x| byte_score(x ^ a))
                        .sum();
                    let sb: f32 = data
                        .iter()
                        .skip(pos)
                        .step_by(key_len)
                        .map(|x| byte_score(x ^ b))
                        .sum();
                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0)
        })
        .collect()
}

fn xor_apply(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect()
}

fn printable_key(key: &[u8]) -> String {
    if key.iter().all(|&b| (0x20..=0x7e).contains(&b)) {
        String::from_utf8_lossy(key).to_string()
    } else {
        hex::encode(key)
    }
}

struct RepeatingXorCrack;

impl Node for RepeatingXorCrack {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = decode_input(in_bytes(i, "data")?, pstr(p, "inputFormat", "Raw/UTF8"))?;
        if data.len() < 2 {
            return Err(CoreError::Parse("输入太短，无法破解重复密钥 XOR".into()));
        }
        let max_len = (pnum(p, "maxKeyLen", 32.0) as usize)
            .clamp(1, 128)
            .min(data.len());
        let top = (pnum(p, "top", 10.0) as usize).clamp(1, 50);
        let mut candidates = Vec::new();
        for len in 1..=max_len {
            let key = xor_key_for_len(&data, len);
            let plain = xor_apply(&data, &key);
            let text = String::from_utf8_lossy(&plain).to_string();
            let score = crack_score(&text);
            candidates.push(CrackCandidate {
                text,
                score,
                note: format!("len={len} keyHex={}", hex::encode(&key)),
                key: format!("key={}", printable_key(&key)),
            });
        }
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let best_bytes = candidates.first().map(|best| {
            let key_text = best
                .note
                .split("keyHex=")
                .nth(1)
                .and_then(|s| hex::decode(s).ok())
                .unwrap_or_default();
            xor_apply(&data, &key_text)
        });
        Ok(finish_candidates(candidates, top, best_bytes))
    }
}

// ------------------------------------------------------------- Rail fence

fn rail_pattern(len: usize, rails: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(len);
    let (mut rail, mut dir) = (0i64, 1i64);
    for _ in 0..len {
        out.push(rail as usize);
        if rail == 0 {
            dir = 1;
        } else if rail == rails as i64 - 1 {
            dir = -1;
        }
        rail += dir;
    }
    out
}

fn rail_decode(s: &str, rails: usize) -> String {
    if rails < 2 {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let pat = rail_pattern(n, rails);
    let mut rail_chars: Vec<Vec<char>> = vec![Vec::new(); rails];
    let mut ci = 0;
    for (r, bucket) in rail_chars.iter_mut().enumerate() {
        for &pr in &pat {
            if pr == r {
                bucket.push(chars[ci]);
                ci += 1;
            }
        }
    }
    let mut pos = vec![0usize; rails];
    let mut out = String::with_capacity(n);
    for &r in &pat {
        out.push(rail_chars[r][pos[r]]);
        pos[r] += 1;
    }
    out
}

struct RailFenceBruteforce;

impl Node for RailFenceBruteforce {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(i, "text")?;
        let max = (pnum(p, "maxRails", 32.0) as usize).clamp(2, 256);
        let top = (pnum(p, "top", 10.0) as usize).clamp(1, 50);
        let candidates = (2..=max.min(input.chars().count().max(2)))
            .map(|rails| {
                let text = rail_decode(input, rails);
                CrackCandidate {
                    score: crack_score(&text),
                    text,
                    note: format!("rails={rails}"),
                    key: format!("key={rails}"),
                }
            })
            .collect();
        Ok(finish_candidates(candidates, top, None))
    }
}

// ------------------------------------------------------------- Affine

fn affine_inv(a: i64) -> Option<i64> {
    (1..26).find(|&x| (a * x).rem_euclid(26) == 1)
}

fn affine_decrypt(s: &str, a: i64, b: i64) -> String {
    let inv = affine_inv(a).unwrap_or(1);
    s.chars()
        .map(|c| {
            let base = if c.is_ascii_lowercase() {
                b'a'
            } else if c.is_ascii_uppercase() {
                b'A'
            } else {
                return c;
            };
            let x = (c as u8 - base) as i64;
            let y = (inv * (x - b)).rem_euclid(26);
            (base + y as u8) as char
        })
        .collect()
}

struct AffineBruteforce;

impl Node for AffineBruteforce {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(i, "text")?;
        let top = (pnum(p, "top", 10.0) as usize).clamp(1, 50);
        let mut candidates = Vec::new();
        for a in 1..26 {
            if affine_inv(a).is_none() {
                continue;
            }
            for b in 0..26 {
                let text = affine_decrypt(input, a, b);
                candidates.push(CrackCandidate {
                    score: crack_score(&text),
                    text,
                    note: format!("a={a} b={b}"),
                    key: format!("key=a:{a},b:{b}"),
                });
            }
        }
        Ok(finish_candidates(candidates, top, None))
    }
}

// ------------------------------------------------------------- Columnar transposition

fn permutations(n: usize, cap: usize) -> Vec<Vec<usize>> {
    fn rec(
        n: usize,
        cap: usize,
        cur: &mut Vec<usize>,
        used: &mut Vec<bool>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if out.len() >= cap {
            return;
        }
        if cur.len() == n {
            out.push(cur.clone());
            return;
        }
        for i in 0..n {
            if used[i] {
                continue;
            }
            used[i] = true;
            cur.push(i);
            rec(n, cap, cur, used, out);
            cur.pop();
            used[i] = false;
        }
    }
    let mut out = Vec::new();
    rec(n, cap, &mut Vec::new(), &mut vec![false; n], &mut out);
    out
}

fn column_lengths(len: usize, cols: usize) -> Vec<usize> {
    let rows = len.div_ceil(cols);
    let rem = len % cols;
    (0..cols)
        .map(|c| if rem == 0 || c < rem { rows } else { rows - 1 })
        .collect()
}

fn columnar_decode(cipher: &str, order: &[usize]) -> String {
    let chars: Vec<char> = cipher.chars().collect();
    let cols = order.len();
    let rows = chars.len().div_ceil(cols);
    let lens = column_lengths(chars.len(), cols);
    let mut columns = vec![Vec::<char>::new(); cols];
    let mut pos = 0usize;
    for &col in order {
        let len = lens[col];
        columns[col] = chars[pos..pos + len].to_vec();
        pos += len;
    }
    let mut out = String::with_capacity(chars.len());
    for r in 0..rows {
        for c in 0..cols {
            if r < columns[c].len() {
                out.push(columns[c][r]);
            }
        }
    }
    out
}

struct ColumnarBruteforce;

impl Node for ColumnarBruteforce {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(i, "text")?;
        let max_cols = (pnum(p, "maxColumns", 7.0) as usize).clamp(2, 8);
        let top = (pnum(p, "top", 10.0) as usize).clamp(1, 50);
        let mut candidates = Vec::new();
        for cols in 2..=max_cols.min(input.chars().count().max(2)) {
            for order in permutations(cols, 50_000) {
                let text = columnar_decode(input, &order);
                let order_text = order
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                candidates.push(CrackCandidate {
                    score: crack_score(&text),
                    text,
                    note: format!("cols={cols} order=[{order_text}]"),
                    key: format!("key={order_text}"),
                });
            }
        }
        Ok(finish_candidates(candidates, top, None))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let top = || ParamSpec::number("top", "候选数", 1.0, 50.0, 1.0, 10.0);
    reg.register(
        desc(
            "vigenere_analyze",
            CRYPTO,
            "维吉尼亚自动分析",
            ROSE,
            vec![t_in()],
            vec![
                req("best", "最佳明文", PortType::Text),
                opt("key", "密钥", PortType::Text),
                opt("text", "候选摘要", PortType::Text),
                opt("candidates", "候选", PortType::Candidates),
                opt("json", "结构", PortType::Json),
            ],
            vec![
                ParamSpec::number("maxKeyLen", "最大密钥长度", 1.0, 64.0, 1.0, 16.0),
                top(),
            ],
        ),
        Arc::new(|| Arc::new(VigenereAnalyze)),
    );
    reg.register(
        desc(
            "repeating_xor_crack",
            ENC,
            "重复密钥 XOR 破解",
            PURPLE,
            vec![req("data", "密文", PortType::Any)],
            vec![
                req("best", "最佳明文", PortType::Text),
                opt("key", "密钥", PortType::Text),
                opt("text", "候选摘要", PortType::Text),
                opt("bytes", "最佳字节", PortType::Bytes),
                opt("candidates", "候选", PortType::Candidates),
                opt("json", "结构", PortType::Json),
            ],
            vec![
                ParamSpec::select(
                    "inputFormat",
                    "输入格式",
                    &["Raw/UTF8", "Hex", "Base64"],
                    "Raw/UTF8",
                ),
                ParamSpec::number("maxKeyLen", "最大密钥长度", 1.0, 128.0, 1.0, 32.0),
                top(),
            ],
        ),
        Arc::new(|| Arc::new(RepeatingXorCrack)),
    );
    reg.register(
        desc(
            "rail_fence_bruteforce",
            CRYPTO,
            "栅栏密码爆破",
            ROSE,
            vec![t_in()],
            vec![
                req("best", "最佳明文", PortType::Text),
                opt("key", "栏数", PortType::Text),
                opt("text", "候选摘要", PortType::Text),
                opt("candidates", "候选", PortType::Candidates),
                opt("json", "结构", PortType::Json),
            ],
            vec![
                ParamSpec::number("maxRails", "最大栏数", 2.0, 256.0, 1.0, 32.0),
                top(),
            ],
        ),
        Arc::new(|| Arc::new(RailFenceBruteforce)),
    );
    reg.register(
        desc(
            "affine_bruteforce",
            CRYPTO,
            "仿射密码爆破",
            ROSE,
            vec![t_in()],
            vec![
                req("best", "最佳明文", PortType::Text),
                opt("key", "参数", PortType::Text),
                opt("text", "候选摘要", PortType::Text),
                opt("candidates", "候选", PortType::Candidates),
                opt("json", "结构", PortType::Json),
            ],
            vec![top()],
        ),
        Arc::new(|| Arc::new(AffineBruteforce)),
    );
    reg.register(
        desc(
            "columnar_bruteforce",
            CRYPTO,
            "列置换爆破",
            ROSE,
            vec![t_in()],
            vec![
                req("best", "最佳明文", PortType::Text),
                opt("key", "列顺序", PortType::Text),
                opt("text", "候选摘要", PortType::Text),
                opt("candidates", "候选", PortType::Candidates),
                opt("json", "结构", PortType::Json),
            ],
            vec![
                ParamSpec::number("maxColumns", "最大列数", 2.0, 8.0, 1.0, 7.0),
                top(),
            ],
        ),
        Arc::new(|| Arc::new(ColumnarBruteforce)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    fn run_text(id: &str, text: &str, params: serde_json::Value) -> PortMap {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(text.into()));
        GraphExecutor::run_node(
            &default_registry(),
            id,
            &i,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    fn run_data(id: &str, data: Vec<u8>, params: serde_json::Value) -> PortMap {
        let mut i = PortMap::new();
        i.insert(
            "data".into(),
            PortValue::Bytes(Arc::from(data.into_boxed_slice())),
        );
        GraphExecutor::run_node(
            &default_registry(),
            id,
            &i,
            &params,
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    fn vigenere_encrypt(text: &str, key: &[u8]) -> String {
        let mut ki = 0usize;
        text.chars()
            .map(|c| {
                let base = if c.is_ascii_lowercase() {
                    b'a'
                } else if c.is_ascii_uppercase() {
                    b'A'
                } else {
                    return c;
                };
                let x = c as u8 - base;
                let k = key[ki % key.len()];
                ki += 1;
                (base + (x + k) % 26) as char
            })
            .collect()
    }

    fn rail_encode(s: &str, rails: usize) -> String {
        let chars: Vec<char> = s.chars().collect();
        let pat = rail_pattern(chars.len(), rails);
        let mut fence = vec![String::new(); rails];
        for (i, &c) in chars.iter().enumerate() {
            fence[pat[i]].push(c);
        }
        fence.concat()
    }

    fn affine_encrypt(s: &str, a: i64, b: i64) -> String {
        s.chars()
            .map(|c| {
                let base = if c.is_ascii_lowercase() {
                    b'a'
                } else if c.is_ascii_uppercase() {
                    b'A'
                } else {
                    return c;
                };
                let x = (c as u8 - base) as i64;
                (base + ((a * x + b).rem_euclid(26) as u8)) as char
            })
            .collect()
    }

    fn columnar_encode(plain: &str, order: &[usize]) -> String {
        let chars: Vec<char> = plain.chars().collect();
        let cols = order.len();
        let rows = chars.len().div_ceil(cols);
        let mut grid = vec![vec![None; cols]; rows];
        for (i, ch) in chars.iter().copied().enumerate() {
            grid[i / cols][i % cols] = Some(ch);
        }
        let mut out = String::new();
        for &col in order {
            for row in &grid {
                if let Some(ch) = row[col] {
                    out.push(ch);
                }
            }
        }
        out
    }

    #[test]
    fn rail_and_affine_bruteforce_recover_plaintext() {
        let plain = "flag{classic_cipher_attack} this sentence is readable";
        let rail = rail_encode(plain, 4);
        let out = run_text(
            "rail_fence_bruteforce",
            &rail,
            json!({ "maxRails": 8, "top": 3 }),
        );
        assert!(matches!(out.get("best"), Some(PortValue::Text(t)) if t.contains("flag{classic")));

        let aff = affine_encrypt(plain, 5, 8);
        let out = run_text("affine_bruteforce", &aff, json!({ "top": 3 }));
        assert!(matches!(out.get("best"), Some(PortValue::Text(t)) if t.contains("flag{classic")));
    }

    #[test]
    fn repeating_xor_crack_recovers_key() {
        let plain = b"flag{repeating_xor_attack} this is a longer english sentence for scoring";
        let key = b"ICE";
        let cipher: Vec<u8> = plain
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        let out = run_data(
            "repeating_xor_crack",
            cipher,
            json!({ "maxKeyLen": 8, "top": 5 }),
        );
        assert!(
            matches!(out.get("best"), Some(PortValue::Text(t)) if t.contains("flag{repeating"))
        );
    }

    #[test]
    fn columnar_bruteforce_recovers_plaintext() {
        let plain = "flag{columnar_transposition_attack} readable english text";
        let cipher = columnar_encode(plain, &[2, 0, 1]);
        let out = run_text(
            "columnar_bruteforce",
            &cipher,
            json!({ "maxColumns": 4, "top": 5 }),
        );
        assert!(matches!(out.get("best"), Some(PortValue::Text(t)) if t.contains("flag{columnar")));
    }

    #[test]
    fn vigenere_analyze_produces_readable_candidate() {
        let plain =
            "the quick brown fox jumps over the lazy dog and the flag is flag{vigenere_attack}";
        let cipher = vigenere_encrypt(plain, &[11, 4, 12, 14, 13]);
        let out = run_text(
            "vigenere_analyze",
            &cipher,
            json!({ "maxKeyLen": 10, "top": 5 }),
        );
        assert!(matches!(out.get("text"), Some(PortValue::Text(t)) if t.contains("flag")));
    }
}
