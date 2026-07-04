//! 正则路由：按 正则0→3 依次匹配输入文本，命中就把原文送到对应输出端口（其余为 None），
//! 都不中送 `default`。一步完成「分类 + 分流」，强于只认数字索引的 `多路分支`。
use regex::RegexBuilder;

use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        let ic = pbool(p, "ignoreCase", false);
        let mut matched: Option<usize> = None;
        for idx in 0..4 {
            let pat = pstr(p, &format!("p{idx}"), "");
            if pat.is_empty() {
                continue;
            }
            let re = RegexBuilder::new(pat)
                .case_insensitive(ic)
                .build()
                .map_err(|e| CoreError::Parse(format!("正则{idx}无效: {e}")))?;
            if re.is_match(text) {
                matched = Some(idx);
                break;
            }
        }
        let mut m = PortMap::new();
        for idx in 0..4 {
            let v = if matched == Some(idx) {
                PortValue::Text(text.to_string())
            } else {
                PortValue::None
            };
            m.insert(format!("out{idx}"), v);
        }
        m.insert(
            "default".into(),
            if matched.is_none() {
                PortValue::Text(text.to_string())
            } else {
                PortValue::None
            },
        );
        m.insert(
            "index".into(),
            PortValue::Number(matched.map(|x| x as f64).unwrap_or(-1.0)),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "regex_router",
            CTL,
            "正则路由",
            AMBER,
            vec![req("text", "输入", PortType::Text)],
            vec![
                req("out0", "分支0", PortType::Text),
                req("out1", "分支1", PortType::Text),
                req("out2", "分支2", PortType::Text),
                req("out3", "分支3", PortType::Text),
                req("default", "默认", PortType::Text),
                opt("index", "命中序号", PortType::Number),
            ],
            vec![
                ParamSpec::text("p0", "正则0", "", false),
                ParamSpec::text("p1", "正则1", "", false),
                ParamSpec::text("p2", "正则2", "", false),
                ParamSpec::text("p3", "正则3", "", false),
                ParamSpec::toggle("ignoreCase", "忽略大小写", false),
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

    fn route(text: &str) -> (PortMap,) {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(text.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "regex_router",
            &i,
            &serde_json::json!({ "p0": "^\\d+$", "p1": "^[a-f0-9]+$" }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        (out,)
    }

    #[test]
    fn routes_to_first_match_else_default() {
        // digits → out0
        let (o,) = route("12345");
        assert!(matches!(o.get("out0"), Some(PortValue::Text(s)) if s == "12345"));
        assert!(matches!(o.get("out1"), Some(PortValue::None)));
        assert!(matches!(o.get("default"), Some(PortValue::None)));
        assert!(matches!(o.get("index"), Some(PortValue::Number(n)) if *n == 0.0));

        // hex letters (not all digits) → out1
        let (o,) = route("abc");
        assert!(matches!(o.get("out1"), Some(PortValue::Text(s)) if s == "abc"));

        // neither → default
        let (o,) = route("hello world!");
        assert!(matches!(o.get("default"), Some(PortValue::Text(s)) if s == "hello world!"));
        assert!(matches!(o.get("index"), Some(PortValue::Number(n)) if *n == -1.0));
    }
}
