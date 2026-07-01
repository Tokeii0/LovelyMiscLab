//! Reduce a list to a single text by joining with a separator.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let list = in_list(inputs, "list")?;
        let sep = match pstr(params, "sep", "换行") {
            "空格" => " ",
            "逗号" => ",",
            "无" => "",
            _ => "\n",
        };
        Ok(out_text(list.join(sep)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "join_list",
            CTL,
            "列表合并",
            AMBER,
            vec![req("list", "列表", PortType::StringList)],
            vec![req("text", "文本", PortType::Text)],
            vec![ParamSpec::select("sep", "分隔符", &["换行", "逗号", "空格", "无"], "换行")],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
