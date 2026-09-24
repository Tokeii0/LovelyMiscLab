//! Character frequency analysis → a sorted table.
use std::collections::HashMap;

use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let total = text.chars().count() as f64;
        let mut counts: HashMap<char, usize> = HashMap::new();
        for c in text.chars() {
            *counts.entry(c).or_default() += 1;
        }
        let mut items: Vec<(char, usize)> = counts.into_iter().collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let table = items
            .iter()
            .map(|(c, n)| {
                let shown = match c {
                    ' ' => "␠".to_string(),
                    '\n' => "\\n".to_string(),
                    '\t' => "\\t".to_string(),
                    c => c.to_string(),
                };
                let pct = if total > 0.0 {
                    *n as f64 / total * 100.0
                } else {
                    0.0
                };
                format!("{shown}\t{n}\t{pct:.1}%")
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok(out_text(table))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "char_frequency",
            UTIL,
            "字符频率",
            AMBER,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
