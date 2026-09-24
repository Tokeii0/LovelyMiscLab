//! Shannon entropy (bits per byte, 0–8) of the input.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let mut freq = [0usize; 256];
        for &b in &data {
            freq[b as usize] += 1;
        }
        let len = data.len() as f64;
        let e = if len == 0.0 {
            0.0
        } else {
            -freq
                .iter()
                .filter(|&&c| c > 0)
                .map(|&c| {
                    let p = c as f64 / len;
                    p * p.log2()
                })
                .sum::<f64>()
        };
        let mut m = PortMap::new();
        m.insert(
            "entropy".to_string(),
            PortValue::Number((e * 10000.0).round() / 10000.0),
        );
        m.insert(
            "text".to_string(),
            PortValue::Text(format!("{e:.4} bits/byte")),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "entropy",
            UTIL,
            "香农熵",
            AMBER,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("entropy", "熵", PortType::Number),
                opt("text", "说明", PortType::Text),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
