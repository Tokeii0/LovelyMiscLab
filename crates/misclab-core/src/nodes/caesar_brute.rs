//! 凯撒爆破：一次试完 25 个位移，按英文字母频率打分排序，最像英文的排在 best。
use super::prelude::*;

fn shift(text: &str, n: u8) -> String {
    text.chars()
        .map(|c| match c {
            'a'..='z' => (((c as u8 - b'a' + n) % 26) + b'a') as char,
            'A'..='Z' => (((c as u8 - b'A' + n) % 26) + b'A') as char,
            o => o,
        })
        .collect()
}

/// 英文字母频率（%），a..z。用于给候选打分——真正的英文明文含更多高频字母。
const FREQ: [f32; 26] = [
    8.2, 1.5, 2.8, 4.3, 12.7, 2.2, 2.0, 6.1, 7.0, 0.15, 0.77, 4.0, 2.4, 6.7, 7.5, 1.9, 0.095, 6.0,
    6.3, 9.1, 2.8, 0.98, 2.4, 0.15, 2.0, 0.074,
];

/// 平均字母频率得分（越高越像英文）。
fn freq_score(s: &str) -> f32 {
    let mut sum = 0.0;
    let mut n = 0u32;
    for c in s.chars() {
        if c.is_ascii_alphabetic() {
            sum += FREQ[(c.to_ascii_lowercase() as u8 - b'a') as usize];
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        sum / n as f32
    }
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        let mut cands: Vec<ScoredString> = (1..26u8)
            .map(|n| {
                let s = shift(text, n);
                ScoredString {
                    score: freq_score(&s),
                    text: s,
                    note: Some(format!("位移 {n}")),
                }
            })
            .collect();
        cands.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let best = cands.first().map(|c| c.text.clone()).unwrap_or_default();

        let mut m = PortMap::new();
        m.insert("best".into(), PortValue::Text(best));
        m.insert("candidates".into(), PortValue::Candidates(cands));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "caesar_brute",
            CRYPTO,
            "凯撒爆破",
            ROSE,
            vec![t_in()],
            vec![
                req("best", "最佳", PortType::Text),
                opt("candidates", "候选(带打分)", PortType::Candidates),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    #[test]
    fn finds_plaintext() {
        let plain = "hello world this is a secret";
        let ct = shift(plain, 3);
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(ct));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "caesar_brute",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        // The real plaintext must be among the 25 candidates...
        let cands = match out.get("candidates") {
            Some(PortValue::Candidates(v)) => v.clone(),
            o => panic!("{o:?}"),
        };
        assert!(cands.iter().any(|c| c.text == plain), "plaintext not in candidates");
        // ...and the frequency score should surface it as best.
        assert!(matches!(out.get("best"), Some(PortValue::Text(s)) if s == plain));
    }
}
