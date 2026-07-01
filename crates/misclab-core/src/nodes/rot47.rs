//! ROT47 — rotate the printable ASCII range (33–126) by 47. Self-inverse.
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
            .map(|c| {
                let o = c as u32;
                if (33..=126).contains(&o) {
                    char::from_u32(33 + (o - 33 + 47) % 94).unwrap()
                } else {
                    c
                }
            })
            .collect();
        Ok(out_text(s))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc("rot47", CRYPTO, "ROT47", ROSE, vec![t_in()], vec![t_out()], vec![]),
        Arc::new(|| Arc::new(N)),
    );
}
