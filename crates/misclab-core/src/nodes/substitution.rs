//! Monoalphabetic substitution cipher — map a plaintext alphabet to a cipher one.
use std::collections::HashMap;

use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let from: Vec<char> = pstr(params, "from", "ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .chars()
            .collect();
        let to: Vec<char> = pstr(params, "to", "").chars().collect();
        let map: HashMap<char, char> = from.iter().zip(to.iter()).map(|(&a, &b)| (a, b)).collect();

        let out: String = in_text(inputs, "text")?
            .chars()
            .map(|c| {
                if let Some(&m) = map.get(&c) {
                    m
                } else if let Some(&m) = map.get(&c.to_ascii_uppercase()) {
                    // Case-insensitive fallback: preserve the input's case.
                    if c.is_lowercase() {
                        m.to_ascii_lowercase()
                    } else {
                        m
                    }
                } else {
                    c
                }
            })
            .collect();
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "substitution",
            CRYPTO,
            "替换密码",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::text("from", "明文字母表", "ABCDEFGHIJKLMNOPQRSTUVWXYZ", false),
                ParamSpec::text("to", "密文字母表", "", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
