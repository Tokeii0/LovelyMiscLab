//! bcrypt password hashing — compute a salted hash or verify a password
//! against an existing `$2b$…` digest.
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let pw = in_text(inputs, "text")?;
        match pstr(p, "operation", "哈希") {
            "校验" => {
                let target = pstr(p, "hash", "");
                let ok = bcrypt::verify(pw, target)
                    .map_err(|e| CoreError::Other(format!("bcrypt: {e}")))?;
                let mut m = PortMap::new();
                m.insert("result".to_string(), PortValue::Bool(ok));
                m.insert(
                    "text".to_string(),
                    PortValue::Text(if ok { "匹配 ✓" } else { "不匹配 ✗" }.to_string()),
                );
                Ok(m)
            }
            _ => {
                let cost = pnum(p, "cost", 10.0).clamp(4.0, 15.0) as u32;
                let h =
                    bcrypt::hash(pw, cost).map_err(|e| CoreError::Other(format!("bcrypt: {e}")))?;
                Ok(out_text(h))
            }
        }
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "bcrypt",
            HASH,
            "bcrypt",
            CYAN,
            vec![req("text", "口令", PortType::Text)],
            vec![
                req("text", "结果", PortType::Text),
                opt("result", "匹配", PortType::Bool),
            ],
            vec![
                ParamSpec::select("operation", "操作", &["哈希", "校验"], "哈希"),
                ParamSpec::number("cost", "代价(4-15)", 4.0, 15.0, 1.0, 10.0),
                ParamSpec::text("hash", "校验目标 hash", "", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
