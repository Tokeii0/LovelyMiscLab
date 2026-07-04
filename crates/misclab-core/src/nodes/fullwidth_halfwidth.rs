//! 全角/半角转换：全角 ASCII（Ａ０！等，U+FF01..FF5E）↔ 半角（U+21..7E），全角空格
//! U+3000 ↔ 半角空格。CJK 场景常把 flag 塞成全角。
use super::prelude::*;

fn full_to_half(c: char) -> char {
    match c as u32 {
        0x3000 => ' ',
        cp @ 0xFF01..=0xFF5E => char::from_u32(cp - 0xFEE0).unwrap_or(c),
        _ => c,
    }
}

fn half_to_full(c: char) -> char {
    match c as u32 {
        0x20 => '\u{3000}',
        cp @ 0x21..=0x7E => char::from_u32(cp + 0xFEE0).unwrap_or(c),
        _ => c,
    }
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
        let out: String = if pstr(p, "direction", "全角→半角") == "半角→全角" {
            text.chars().map(half_to_full).collect()
        } else {
            text.chars().map(full_to_half).collect()
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "fullwidth_halfwidth",
            CHARSET,
            "全角/半角",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select("direction", "方向", &["全角→半角", "半角→全角"], "全角→半角")],
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
            "fullwidth_halfwidth",
            &i,
            &serde_json::json!({ "direction": dir }),
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
    fn converts_both_ways() {
        let full = "ｆｌａｇ｛１２３｝";
        assert_eq!(run(full, "全角→半角"), "flag{123}");
        assert_eq!(run("flag{123}", "半角→全角"), full);
    }
}
