//! Find & replace using a regular expression (supports $1 capture groups).
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let s = in_text(inputs, "text")?;
        let pattern = pstr(params, "pattern", "");
        if pattern.is_empty() {
            return Ok(out_text(s.to_string()));
        }
        let re =
            regex::Regex::new(pattern).map_err(|e| CoreError::Parse(format!("正则错误: {e}")))?;
        let rep = pstr(params, "replacement", "");
        let out = if pbool(params, "global", true) {
            re.replace_all(s, rep).into_owned()
        } else {
            re.replace(s, rep).into_owned()
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "regex_replace",
            TXT,
            "正则替换",
            TEAL,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::text("pattern", "正则", "", false),
                ParamSpec::text("replacement", "替换为($1 引用分组)", "", false),
                ParamSpec::toggle("global", "全部替换", true),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
