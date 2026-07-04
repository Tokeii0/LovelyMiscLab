//! 键盘移位：把每个字母替换成 QWERTY 键盘上左/右相邻的键。CTF 里常见的「手放错一个键」。
use super::prelude::*;

const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

fn shift_char(c: char, delta: i32, wrap: bool) -> char {
    let lower = c.to_ascii_lowercase();
    for row in ROWS {
        if let Some(pos) = row.find(lower) {
            let len = row.len() as i32;
            let np = if wrap {
                (pos as i32 + delta).rem_euclid(len)
            } else {
                let n = pos as i32 + delta;
                if n < 0 || n >= len {
                    return c;
                }
                n
            };
            let nc = row.as_bytes()[np as usize] as char;
            return if c.is_ascii_uppercase() { nc.to_ascii_uppercase() } else { nc };
        }
    }
    c
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let delta = if pstr(p, "direction", "右移") == "左移" { -1 } else { 1 };
        let wrap = pbool(p, "wrap", false);
        let s: String = in_text(i, "text")?.chars().map(|c| shift_char(c, delta, wrap)).collect();
        Ok(out_text(s))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "keyboard_shift",
            CRYPTO,
            "键盘移位",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::select("direction", "方向", &["右移", "左移"], "右移"),
                ParamSpec::toggle("wrap", "行内环绕", false),
            ],
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

    fn run(text: &str, dir: &str) -> String {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(text.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "keyboard_shift",
            &i,
            &serde_json::json!({ "direction": dir, "wrap": true }),
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
    fn shift_roundtrip() {
        let enc = run("Hello", "右移");
        assert_eq!(run(&enc, "左移"), "Hello");
        // q →(右) w
        assert_eq!(run("q", "右移"), "w");
    }
}
