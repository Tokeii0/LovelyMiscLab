//! Defang / refang IOCs (URLs, IPs, emails) for safe sharing.
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
        let out = if pstr(params, "operation", "defang") == "refang" {
            s.replace("[.]", ".")
                .replace("[://]", "://")
                .replace("hxxp", "http")
                .replace("[at]", "@")
                .replace("[@]", "@")
        } else {
            s.replace("://", "[://]")
                .replace("http", "hxxp")
                .replace('.', "[.]")
                .replace('@', "[at]")
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "defang",
            UTIL,
            "Defang/Refang",
            AMBER,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select(
                "operation",
                "操作",
                &["defang", "refang"],
                "defang",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
