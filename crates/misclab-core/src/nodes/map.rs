//! Map / for-each — apply one transform to every element of a list.
use super::prelude::*;
use super::xform::{apply_transform, TRANSFORMS};

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let list = in_list(inputs, "list")?;
        let op = pstr(params, "op", "大写");
        let out: Result<Vec<String>, CoreError> =
            list.iter().map(|s| apply_transform(op, s)).collect();
        Ok(one("list", PortValue::StringList(out?)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "map",
            CTL,
            "逐项映射",
            AMBER,
            vec![req("list", "列表", PortType::StringList)],
            vec![req("list", "结果", PortType::StringList)],
            vec![ParamSpec::select("op", "操作", TRANSFORMS, "大写")],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
