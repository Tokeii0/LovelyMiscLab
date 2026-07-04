//! NATO 音标字母：A→Alpha … Z→Zulu，数字 0→Zero … 9→Niner。编码/解码互转。
use super::prelude::*;

const NATO: [(char, &str); 36] = [
    ('A', "Alpha"), ('B', "Bravo"), ('C', "Charlie"), ('D', "Delta"), ('E', "Echo"),
    ('F', "Foxtrot"), ('G', "Golf"), ('H', "Hotel"), ('I', "India"), ('J', "Juliett"),
    ('K', "Kilo"), ('L', "Lima"), ('M', "Mike"), ('N', "November"), ('O', "Oscar"),
    ('P', "Papa"), ('Q', "Quebec"), ('R', "Romeo"), ('S', "Sierra"), ('T', "Tango"),
    ('U', "Uniform"), ('V', "Victor"), ('W', "Whiskey"), ('X', "Xray"), ('Y', "Yankee"),
    ('Z', "Zulu"), ('0', "Zero"), ('1', "One"), ('2', "Two"), ('3', "Three"),
    ('4', "Four"), ('5', "Five"), ('6', "Six"), ('7', "Seven"), ('8', "Eight"), ('9', "Niner"),
];

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
            text.to_uppercase()
                .chars()
                .filter_map(|c| NATO.iter().find(|(k, _)| *k == c).map(|(_, w)| *w))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            text.split_whitespace()
                .filter_map(|word| {
                    NATO.iter()
                        .find(|(_, w)| w.eq_ignore_ascii_case(word))
                        .map(|(k, _)| *k)
                })
                .collect()
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "nato_phonetic",
            ENC,
            "NATO音标",
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
            "nato_phonetic",
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
    fn encode_decode() {
        assert_eq!(run("AB1", "编码"), "Alpha Bravo One");
        assert_eq!(run("Alpha Bravo One", "解码"), "AB1");
    }
}
