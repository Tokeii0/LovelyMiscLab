//! JSON beautify / minify.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let v: serde_json::Value = serde_json::from_str(in_text(inputs, "text")?)
            .map_err(|e| CoreError::Parse(format!("JSON 无效: {e}")))?;
        let out = if pstr(params, "operation", "美化") == "压缩" {
            serde_json::to_string(&v)
        } else {
            serde_json::to_string_pretty(&v)
        }
        .map_err(|e| CoreError::Other(e.to_string()))?;
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "json_format",
            UTIL,
            "JSON 格式化",
            CYAN,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select(
                "operation",
                "操作",
                &["美化", "压缩"],
                "美化",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
