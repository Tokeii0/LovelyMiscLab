//! 字符串内容匹配：判断文本是否 包含/等于/开头是/结尾是/正则匹配… 某个值，输出布尔。
//! 常配合 条件门 / 多路分支 使用（把 result 接到条件、text 接到值）。
use regex::RegexBuilder;

use super::prelude::*;

const OPS: &[&str] = &[
    "包含",
    "不包含",
    "等于",
    "不等于",
    "开头是",
    "结尾是",
    "正则匹配",
    "正则不匹配",
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
        let value = pstr(p, "value", "");
        let op = pstr(p, "operation", "包含");
        let ic = pbool(p, "ignoreCase", false);

        let result = if op.starts_with("正则") {
            let re = RegexBuilder::new(value)
                .case_insensitive(ic)
                .build()
                .map_err(|e| CoreError::Parse(format!("正则无效: {e}")))?;
            let m = re.is_match(text);
            if op == "正则不匹配" {
                !m
            } else {
                m
            }
        } else {
            let (hay, needle) = if ic {
                (text.to_lowercase(), value.to_lowercase())
            } else {
                (text.to_string(), value.to_string())
            };
            match op {
                "包含" => hay.contains(&needle),
                "不包含" => !hay.contains(&needle),
                "等于" => hay == needle,
                "不等于" => hay != needle,
                "开头是" => hay.starts_with(&needle),
                "结尾是" => hay.ends_with(&needle),
                _ => false,
            }
        };

        let mut m = PortMap::new();
        m.insert("result".into(), PortValue::Bool(result));
        m.insert("text".into(), PortValue::Text(text.to_string()));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "string_match",
            CTL,
            "字符串匹配",
            AMBER,
            vec![req("text", "输入", PortType::Text)],
            vec![
                req("result", "匹配", PortType::Bool),
                opt("text", "原文", PortType::Text),
            ],
            vec![
                ParamSpec::select("operation", "判断", OPS, "包含"),
                ParamSpec::text("value", "匹配值 / 正则", "", false),
                ParamSpec::toggle("ignoreCase", "忽略大小写", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
