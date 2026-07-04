//! Brainfuck / Ook! 解释器：给源码（+可选标准输入）跑出结果。复用 `braintools` 的解释器，
//! 不重复实现。Ook! 是 Brainfuck 的同构变体，把源码翻译成 BF 后同样交给 `run_bf`。
use super::braintools::run_bf;
use super::prelude::*;

/// 把 Ook! 源码翻译成 Brainfuck。识别所有 `Ook.` `Ook?` `Ook!` 记号并两两配对。
fn ook_to_bf(src: &str) -> Result<String, String> {
    let b = src.as_bytes();
    let mut toks: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if i + 3 < b.len() && &b[i..i + 3] == b"Ook" && matches!(b[i + 3], b'.' | b'?' | b'!') {
            toks.push(b[i + 3]);
            i += 4;
        } else {
            i += 1;
        }
    }
    if toks.is_empty() {
        return Err("未找到任何 Ook! 记号。".into());
    }
    if !toks.len().is_multiple_of(2) {
        return Err("Ook! 记号数为奇数，无法配对。".into());
    }
    let mut bf = String::with_capacity(toks.len() / 2);
    for pair in toks.chunks(2) {
        bf.push(match (pair[0], pair[1]) {
            (b'.', b'?') => '>',
            (b'?', b'.') => '<',
            (b'.', b'.') => '+',
            (b'!', b'!') => '-',
            (b'!', b'.') => '.',
            (b'.', b'!') => ',',
            (b'!', b'?') => '[',
            (b'?', b'!') => ']',
            _ => return Err("非法的 Ook! 记号对。".into()),
        });
    }
    Ok(bf)
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let src = in_text(i, "text")?;
        // 标准输入：优先取 stdin 端口，否则取参数 input。
        let stdin: Vec<u8> = match i.get("stdin") {
            Some(PortValue::Text(s)) => s.as_bytes().to_vec(),
            Some(PortValue::Bytes(bs)) => bs.to_vec(),
            _ => pstr(p, "input", "").as_bytes().to_vec(),
        };
        let bf = match pstr(p, "dialect", "Brainfuck") {
            "Ook!" => ook_to_bf(src).map_err(CoreError::Parse)?,
            _ => src.to_string(),
        };
        let output = run_bf(&bf, &stdin, 20_000_000).map_err(CoreError::Other)?;
        let mut m = PortMap::new();
        m.insert(
            "text".into(),
            PortValue::Text(String::from_utf8_lossy(&output).into_owned()),
        );
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(output.into_boxed_slice())),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "brainfuck",
            ENC,
            "Brainfuck 解释器",
            PURPLE,
            vec![
                req("text", "源码", PortType::Text),
                opt("stdin", "标准输入", PortType::Any),
            ],
            vec![
                req("text", "输出", PortType::Text),
                opt("bytes", "输出字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select("dialect", "方言", &["Brainfuck", "Ook!"], "Brainfuck"),
                ParamSpec::text("input", "标准输入(可选)", "", false),
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

    fn run(src: &str, params: serde_json::Value) -> String {
        let mut inputs = PortMap::new();
        inputs.insert("text".into(), PortValue::Text(src.to_string()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "brainfuck",
            &inputs,
            &params,
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
    fn hello_world() {
        let hw = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
        assert_eq!(run(hw, serde_json::json!({})), "Hello World!\n");
    }

    #[test]
    fn single_char() {
        assert_eq!(run("++++++++[>++++++++<-]>+.", serde_json::json!({})), "A");
    }

    #[test]
    fn ook_translation() {
        // 依次给出 8 个记号对 → 8 条 BF 指令。
        let ook = "Ook. Ook? Ook? Ook. Ook. Ook. Ook! Ook! Ook! Ook. Ook. Ook! Ook! Ook? Ook? Ook!";
        assert_eq!(ook_to_bf(ook).unwrap(), "><+-.,[]");
    }

    #[test]
    fn ook_runs_to_upper_a() {
        // 把打印 'A' 的 BF 程序转成 Ook! 再跑。
        let bf = "++++++++[>++++++++<-]>+.";
        let ook: String = bf
            .chars()
            .map(|c| match c {
                '>' => "Ook. Ook? ",
                '<' => "Ook? Ook. ",
                '+' => "Ook. Ook. ",
                '-' => "Ook! Ook! ",
                '.' => "Ook! Ook. ",
                ',' => "Ook. Ook! ",
                '[' => "Ook! Ook? ",
                ']' => "Ook? Ook! ",
                _ => "",
            })
            .collect();
        assert_eq!(run(&ook, serde_json::json!({"dialect":"Ook!"})), "A");
    }
}
