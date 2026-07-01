//! Vigenère cipher — polyalphabetic Caesar keyed by a repeating word. Non-letters
//! pass through and do not advance the key.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let key: Vec<i64> = pstr(params, "key", "KEY")
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| (c.to_ascii_lowercase() as u8 - b'a') as i64)
            .collect();
        if key.is_empty() {
            return Err(CoreError::Parse("密钥需至少包含一个字母".into()));
        }
        let decrypt = pstr(params, "operation", "加密") == "解密";
        let mut ki = 0usize;

        let s: String = in_text(inputs, "text")?
            .chars()
            .map(|c| {
                let base = if c.is_ascii_lowercase() {
                    b'a'
                } else if c.is_ascii_uppercase() {
                    b'A'
                } else {
                    return c;
                };
                let x = (c as u8 - base) as i64;
                let k = key[ki % key.len()];
                ki += 1;
                let y = if decrypt {
                    (x - k).rem_euclid(26)
                } else {
                    (x + k).rem_euclid(26)
                };
                (base + y as u8) as char
            })
            .collect();
        Ok(out_text(s))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "vigenere",
            CRYPTO,
            "维吉尼亚密码",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::select("operation", "操作", &["加密", "解密"], "加密"),
                ParamSpec::text("key", "密钥(字母)", "KEY", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
