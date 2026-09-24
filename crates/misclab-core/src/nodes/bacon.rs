//! Bacon's cipher (26-letter, A/B) — encode / decode.
use super::prelude::*;

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let out: String = in_text(inputs, "text")?
            .to_uppercase()
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| {
                let idx = c as u8 - b'A';
                (0..5)
                    .rev()
                    .map(|b| if (idx >> b) & 1 == 1 { 'B' } else { 'A' })
                    .collect::<String>()
            })
            .collect();
        Ok(out_text(out))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        // Accept A/B or 0/1; treat first-of-two symbols as A/0.
        let bits: Vec<u8> = in_text(inputs, "text")?
            .chars()
            .filter_map(|c| match c {
                'A' | 'a' | '0' => Some(0),
                'B' | 'b' | '1' => Some(1),
                _ => None,
            })
            .collect();
        let out: String = bits
            .chunks(5)
            .filter(|ch| ch.len() == 5)
            .map(|ch| {
                let idx = ch.iter().fold(0u8, |acc, &b| (acc << 1) | b);
                (b'A' + idx % 26) as char
            })
            .collect();
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "bacon_encode",
            ENC,
            "培根密码编码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "bacon_decode",
            ENC,
            "培根密码解码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
