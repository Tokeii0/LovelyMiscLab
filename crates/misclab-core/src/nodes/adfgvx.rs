//! ADFGVX / ADFGX cipher — Polybius fractionation followed by a keyed
//! columnar transposition. WWI German field cipher.
use super::prelude::*;

fn labels(size: usize) -> &'static [u8] {
    if size == 6 {
        b"ADFGVX"
    } else {
        b"ADFGX"
    }
}

/// Build the size×size Polybius square from a (possibly partial) keyword,
/// padded with the remaining standard alphabet. 5×5 merges J→I and drops digits.
fn build_square(input: &str, size: usize) -> Vec<u8> {
    let full: Vec<u8> = if size == 6 {
        (b'A'..=b'Z').chain(b'0'..=b'9').collect()
    } else {
        (b'A'..=b'Z').filter(|&c| c != b'J').collect()
    };
    let mut sq: Vec<u8> = Vec::with_capacity(size * size);
    let push = |raw: u8, sq: &mut Vec<u8>| {
        let mut c = raw.to_ascii_uppercase();
        if size == 5 && c == b'J' {
            c = b'I';
        }
        if full.contains(&c) && !sq.contains(&c) {
            sq.push(c);
        }
    };
    for b in input.bytes() {
        push(b, &mut sq);
    }
    for &b in &full {
        push(b, &mut sq);
    }
    sq
}

fn clean_plaintext(text: &str, square: &[u8], size: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for ch in text.chars() {
        if !ch.is_ascii_alphanumeric() {
            continue;
        }
        let mut u = (ch as u8).to_ascii_uppercase();
        if size == 5 && u == b'J' {
            u = b'I';
        }
        if square.contains(&u) {
            out.push(u);
        }
    }
    out
}

/// Column read order: column indices sorted by key char, ties by position.
fn key_order(key: &[u8]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..key.len()).collect();
    idx.sort_by(|&a, &b| key[a].cmp(&key[b]).then(a.cmp(&b)));
    idx
}

fn transpose_encrypt(text: &[u8], key: &[u8]) -> Vec<u8> {
    let cols = key.len();
    let mut columns: Vec<Vec<u8>> = vec![Vec::new(); cols];
    for (i, &ch) in text.iter().enumerate() {
        columns[i % cols].push(ch);
    }
    let mut out = Vec::new();
    for &c in &key_order(key) {
        out.extend_from_slice(&columns[c]);
    }
    out
}

fn transpose_decrypt(cipher: &[u8], key: &[u8]) -> Vec<u8> {
    let cols = key.len();
    let n = cipher.len();
    let base = n / cols;
    let extra = n % cols; // first `extra` columns (original order) get one more
    let col_len = |c: usize| if c < extra { base + 1 } else { base };
    let mut columns: Vec<Vec<u8>> = vec![Vec::new(); cols];
    let mut pos = 0;
    for &c in &key_order(key) {
        let len = col_len(c);
        columns[c] = cipher[pos..pos + len].to_vec();
        pos += len;
    }
    let rows = base + if extra > 0 { 1 } else { 0 };
    let mut out = Vec::new();
    for r in 0..rows {
        for col in columns.iter() {
            if let Some(&b) = col.get(r) {
                out.push(b);
            }
        }
    }
    out
}

fn key_bytes(k: &str) -> Vec<u8> {
    k.bytes().filter(|b| b.is_ascii_alphanumeric()).map(|b| b.to_ascii_uppercase()).collect()
}

struct N;
impl Node for N {
    fn run(&self, inputs: &PortMap, p: &serde_json::Value, _c: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let size = if pstr(p, "variant", "ADFGVX (6×6)").contains('6') { 6 } else { 5 };
        let square = build_square(pstr(p, "square", ""), size);
        let lab = labels(size);
        let key = key_bytes(pstr(p, "keyword", "SECRET"));
        if key.is_empty() {
            return Err(CoreError::Parse("转置关键词不能为空".into()));
        }

        let result = if pstr(p, "operation", "加密") == "解密" {
            // keep only label letters, undo transposition, then de-fractionate
            let cipher: Vec<u8> = text.bytes().map(|b| b.to_ascii_uppercase()).filter(|b| lab.contains(b)).collect();
            let pairs = transpose_decrypt(&cipher, &key);
            let mut out = String::new();
            for chunk in pairs.chunks(2) {
                if chunk.len() < 2 {
                    break;
                }
                let r = lab.iter().position(|&x| x == chunk[0]);
                let c = lab.iter().position(|&x| x == chunk[1]);
                if let (Some(r), Some(c)) = (r, c) {
                    if let Some(&ch) = square.get(r * size + c) {
                        out.push(ch as char);
                    }
                }
            }
            out
        } else {
            let pt = clean_plaintext(text, &square, size);
            let mut frac = Vec::with_capacity(pt.len() * 2);
            for &ch in &pt {
                let idx = square.iter().position(|&x| x == ch).unwrap();
                frac.push(lab[idx / size]);
                frac.push(lab[idx % size]);
            }
            String::from_utf8(transpose_encrypt(&frac, &key)).unwrap()
        };
        Ok(out_text(result))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "adfgvx",
            CRYPTO,
            "ADFGVX 密码",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::select("operation", "操作", &["加密", "解密"], "加密"),
                ParamSpec::select("variant", "变体", &["ADFGVX (6×6)", "ADFGX (5×5)"], "ADFGVX (6×6)"),
                ParamSpec::text("keyword", "转置关键词", "SECRET", false),
                ParamSpec::text("square", "方阵关键词(可空)", "", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
