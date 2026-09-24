//! Bifid cipher (5×5 Polybius square, I=J, whole-message period).
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

fn coords(sq: &[char], c: char) -> Option<(usize, usize)> {
    let c = if c == 'J' { 'I' } else { c };
    sq.iter().position(|&x| x == c).map(|i| (i / 5, i % 5))
}

fn clean_coords(sq: &[char], text: &str) -> Vec<(usize, usize)> {
    text.to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .filter_map(|c| coords(sq, c))
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
        let cs = clean_coords(&sq, in_text(inputs, "text")?);
        let combined: Vec<usize> = cs
            .iter()
            .map(|c| c.0)
            .chain(cs.iter().map(|c| c.1))
            .collect();
        let out: String = combined
            .chunks(2)
            .filter(|ch| ch.len() == 2)
            .map(|ch| sq[ch[0] * 5 + ch[1]])
            .collect();
        Ok(out_text(out))
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
        let cs = clean_coords(&sq, in_text(inputs, "text")?);
        let seq: Vec<usize> = cs.iter().flat_map(|&(r, c)| [r, c]).collect();
        let half = seq.len() / 2;
        let out: String = (0..half).map(|i| sq[seq[i] * 5 + seq[half + i]]).collect();
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let kw = || ParamSpec::text("keyword", "关键词", "", false);
    reg.register(
        desc(
            "bifid_encode",
            CRYPTO,
            "Bifid 加密",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![kw()],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "bifid_decode",
            CRYPTO,
            "Bifid 解密",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![kw()],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
