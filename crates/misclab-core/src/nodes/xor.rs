use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?;
        let key = pstr(params, "key", "");
        if key.is_empty() {
            return Ok(out_text(input.to_string()));
        }
        let kb = key.as_bytes();
        let xored: Vec<u8> = input
            .as_bytes()
            .iter()
            .enumerate()
            .map(|(idx, b)| b ^ kb[idx % kb.len()])
            .collect();
        Ok(out_text(String::from_utf8_lossy(&xored).into_owned()))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "xor",
            ENC,
            "XOR",
            PURPLE,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::text("key", "密钥", "", false)],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
