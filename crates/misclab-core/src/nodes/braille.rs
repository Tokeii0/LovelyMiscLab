//! Braille (Grade-1 English) — encode / decode letters to Unicode braille cells.
use super::prelude::*;

const TABLE: &[(char, char)] = &[
    ('a', '⠁'),
    ('b', '⠃'),
    ('c', '⠉'),
    ('d', '⠙'),
    ('e', '⠑'),
    ('f', '⠋'),
    ('g', '⠛'),
    ('h', '⠓'),
    ('i', '⠊'),
    ('j', '⠚'),
    ('k', '⠅'),
    ('l', '⠇'),
    ('m', '⠍'),
    ('n', '⠝'),
    ('o', '⠕'),
    ('p', '⠏'),
    ('q', '⠟'),
    ('r', '⠗'),
    ('s', '⠎'),
    ('t', '⠞'),
    ('u', '⠥'),
    ('v', '⠧'),
    ('w', '⠺'),
    ('x', '⠭'),
    ('y', '⠽'),
    ('z', '⠵'),
    (' ', '⠀'),
];

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let out: String = in_text(inputs, "text")?
            .to_lowercase()
            .chars()
            .map(|c| {
                TABLE
                    .iter()
                    .find(|(a, _)| *a == c)
                    .map(|(_, b)| *b)
                    .unwrap_or(c)
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
        let out: String = in_text(inputs, "text")?
            .chars()
            .map(|c| {
                TABLE
                    .iter()
                    .find(|(_, b)| *b == c)
                    .map(|(a, _)| *a)
                    .unwrap_or(c)
            })
            .collect();
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "braille_encode",
            ENC,
            "盲文编码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "braille_decode",
            ENC,
            "盲文解码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
