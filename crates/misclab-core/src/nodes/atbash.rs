//! Atbash — mirror the Latin alphabet (a↔z, A↔Z). Self-inverse.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let s: String = in_text(inputs, "text")?
            .chars()
            .map(|c| match c {
                'a'..='z' => (b'z' - (c as u8 - b'a')) as char,
                'A'..='Z' => (b'Z' - (c as u8 - b'A')) as char,
                o => o,
            })
            .collect();
        Ok(out_text(s))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc("atbash", CRYPTO, "Atbash", ROSE, vec![t_in()], vec![t_out()], vec![]),
        Arc::new(|| Arc::new(N)),
    );
}
