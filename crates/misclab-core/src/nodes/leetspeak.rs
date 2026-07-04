//! Leetspeak(1337)：字母↔数字/符号常见替换。解码有歧义（1 既像 i 也像 l），取常见映射。
use super::prelude::*;

fn encode(text: &str) -> String {
    text.chars()
        .map(|c| match c.to_ascii_lowercase() {
            'a' => '4',
            'b' => '8',
            'e' => '3',
            'g' => '6',
            'i' => '1',
            'l' => '1',
            'o' => '0',
            's' => '5',
            't' => '7',
            'z' => '2',
            _ => c,
        })
        .collect()
}

fn decode(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '4' => 'a',
            '8' => 'b',
            '3' => 'e',
            '6' => 'g',
            '1' => 'i',
            '0' => 'o',
            '5' => 's',
            '7' => 't',
            '2' => 'z',
            _ => c,
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
            "leetspeak",
            ENC,
            "Leetspeak",
            BLUE,
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
            "leetspeak",
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
    fn roundtrip_unambiguous() {
        // "beast" has no i/l ambiguity → clean round-trip.
        assert_eq!(run("beast", "编码"), "83457");
        assert_eq!(run("83457", "解码"), "beast");
    }
}
