//! Playfair cipher (5×5 square, I=J) — digraph substitution.
use super::prelude::*;

fn square(keyword: &str) -> Vec<char> {
    let mut seen = [false; 26];
    let mut sq = Vec::with_capacity(25);
    for c in keyword.to_uppercase().chars().chain('A'..='Z') {
        let c = if c == 'J' { 'I' } else { c };
        if c.is_ascii_uppercase() {
            let idx = (c as u8 - b'A') as usize;
            if !seen[idx] {
                seen[idx] = true;
                sq.push(c);
            }
        }
    }
    sq
}

fn pos(sq: &[char], c: char) -> (usize, usize) {
    let c = if c == 'J' { 'I' } else { c };
    let i = sq.iter().position(|&x| x == c).unwrap_or(0);
    (i / 5, i % 5)
}

fn clean(text: &str) -> Vec<char> {
    text.to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| if c == 'J' { 'I' } else { c })
        .collect()
}

/// Split into digraphs, inserting a filler between doubled letters / for odd tails.
fn encrypt_pairs(letters: &[char]) -> Vec<(char, char)> {
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < letters.len() {
        let a = letters[i];
        let filler = if a == 'X' { 'Z' } else { 'X' };
        match letters.get(i + 1) {
            Some(&b) if b != a => {
                pairs.push((a, b));
                i += 2;
            }
            _ => {
                pairs.push((a, filler));
                i += 1;
            }
        }
    }
    pairs
}

fn transform(sq: &[char], pairs: &[(char, char)], dir: i32) -> String {
    let shift = |x: usize| ((x as i32 + dir + 5) % 5) as usize;
    pairs
        .iter()
        .flat_map(|&(a, b)| {
            let (r1, c1) = pos(sq, a);
            let (r2, c2) = pos(sq, b);
            let (nr1, nc1, nr2, nc2) = if r1 == r2 {
                (r1, shift(c1), r2, shift(c2))
            } else if c1 == c2 {
                (shift(r1), c1, shift(r2), c2)
            } else {
                (r1, c2, r2, c1)
            };
            [sq[nr1 * 5 + nc1], sq[nr2 * 5 + nc2]]
        })
        .collect()
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let sq = square(pstr(params, "keyword", ""));
        let pairs = encrypt_pairs(&clean(in_text(inputs, "text")?));
        Ok(out_text(transform(&sq, &pairs, 1)))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let sq = square(pstr(params, "keyword", ""));
        let letters = clean(in_text(inputs, "text")?);
        let pairs: Vec<(char, char)> = letters
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| (c[0], c[1]))
            .collect();
        Ok(out_text(transform(&sq, &pairs, -1)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let kw = || ParamSpec::text("keyword", "关键词", "", false);
    reg.register(
        desc(
            "playfair_encode",
            CRYPTO,
            "Playfair 加密",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![kw()],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "playfair_decode",
            CRYPTO,
            "Playfair 解密",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![kw()],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
