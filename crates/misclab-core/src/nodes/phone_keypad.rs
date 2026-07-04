//! 手机键盘 / 多击(T9)：2→abc … 9→wxyz，字母=数字重复(第几个)次。CTF 常见「227→CAR」。
//! 解码把空格分隔的同数字组还原（`44 33 555 555 666` → hello）；编码反之。
use super::prelude::*;

const KEYS: [(char, &str); 9] = [
    ('0', " "),
    ('2', "abc"),
    ('3', "def"),
    ('4', "ghi"),
    ('5', "jkl"),
    ('6', "mno"),
    ('7', "pqrs"),
    ('8', "tuv"),
    ('9', "wxyz"),
];

fn encode(text: &str) -> String {
    let mut groups = Vec::new();
    for ch in text.to_lowercase().chars() {
        if let Some((digit, letters)) = KEYS.iter().find(|(_, ls)| ls.contains(ch)) {
            let idx = letters.find(ch).unwrap() + 1;
            groups.push(digit.to_string().repeat(idx));
        }
    }
    groups.join(" ")
}

fn decode(text: &str) -> String {
    text.split_whitespace()
        .filter_map(|tok| {
            let d = tok.chars().next()?;
            let (_, letters) = KEYS.iter().find(|(k, _)| *k == d)?;
            letters.chars().nth(tok.len() - 1)
        })
        .collect()
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        let out = if pstr(p, "operation", "解码") == "编码" {
            encode(text)
        } else {
            decode(text)
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "phone_keypad",
            CRYPTO,
            "手机键盘(T9)",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select("operation", "操作", &["解码", "编码"], "解码")],
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

    fn run(text: &str, op: &str) -> String {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(text.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "phone_keypad",
            &i,
            &serde_json::json!({ "operation": op }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("text") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn roundtrip_and_known() {
        assert_eq!(run("44 33 555 555 666", "解码"), "hello");
        assert_eq!(run("hello", "编码"), "44 33 555 555 666");
    }
}
