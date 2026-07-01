use super::prelude::*;

/// Try all 256 single-byte XOR keys, ranked by printable ratio — the "loop"
/// pattern done as a node (a DAG can't express real loops).
struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?.as_bytes().to_vec();
        let mut candidates: Vec<ScoredString> = (0u8..=255)
            .map(|k| {
                let decoded: Vec<u8> = input.iter().map(|b| b ^ k).collect();
                let text = String::from_utf8_lossy(&decoded).into_owned();
                let score = english_score(&text);
                ScoredString {
                    text,
                    score,
                    note: Some(format!("key=0x{k:02x}")),
                }
            })
            .collect();
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(16);

        let best = candidates.first().map(|c| c.text.clone()).unwrap_or_default();
        let mut out = PortMap::new();
        out.insert("best".to_string(), PortValue::Text(best));
        out.insert("candidates".to_string(), PortValue::Candidates(candidates));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "xor_bruteforce",
            ENC,
            "XOR 单字节爆破",
            PURPLE,
            vec![t_in()],
            vec![
                req("best", "最佳", PortType::Text),
                opt("candidates", "候选", PortType::Candidates),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
