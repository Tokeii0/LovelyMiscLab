//! Remove whitespace / specific characters from text.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let s = in_text(inputs, "text")?;
        let out: String = match pstr(params, "mode", "全部空白") {
            "空格" => s.chars().filter(|&c| c != ' ').collect(),
            "换行" => s.chars().filter(|&c| c != '\n' && c != '\r').collect(),
            "制表符" => s.chars().filter(|&c| c != '\t').collect(),
            "非可见字符" => s
                .chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .collect(),
            _ => s.chars().filter(|c| !c.is_whitespace()).collect(),
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "remove_whitespace",
            TXT,
            "去除空白",
            TEAL,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select(
                "mode",
                "去除",
                &["全部空白", "空格", "换行", "制表符", "非可见字符"],
                "全部空白",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
