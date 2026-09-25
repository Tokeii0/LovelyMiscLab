//! 通用口令爆破：用字典驱动**任意目标节点**逐个试口令，按判据认定命中。
//!
//! 核心洞察：错误口令通常让目标节点**报错**（cloacked-pixel 填充失败、压缩包解压失败、
//! AES padding 失败…），所以「无报错即命中」是最通用的判据；对不报错只出乱码的目标
//! （如古典密码），改用「正则命中」或「可打印文本」判据。
//!
//! 目标节点通过其 descriptor id 指定；`data` 输入转发给目标的首个必填输入端口，`extraParams`
//! 可补充目标的其它参数（JSON）。等价于对目标节点做一层「字典 → 设口令 → 跑 → 判定」循环。
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::graph::executor::GraphExecutor;
use crate::progress::NullSink;

use super::prelude::*;

/// 把一个端口值转成可读字符串（供判据/输出用）。
fn pv_to_string(v: &PortValue) -> String {
    match v {
        PortValue::Text(s) => s.clone(),
        PortValue::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
        PortValue::Number(n) => n.to_string(),
        PortValue::Bool(b) => b.to_string(),
        PortValue::StringList(v) => v.join("\n"),
        _ => String::new(),
    }
}

/// 选一个输出端口的文本表示：优先指定端口，否则 text → bytes → 任意。
fn checked_text(out: &PortMap, port: &str) -> String {
    if !port.is_empty() {
        return out.get(port).map(pv_to_string).unwrap_or_default();
    }
    if let Some(v) = out.get("text").or_else(|| out.get("bytes")) {
        return pv_to_string(v);
    }
    out.values().next().map(pv_to_string).unwrap_or_default()
}

/// 大部分是可打印字符（含常见空白与非 ASCII 文字）且非空。
fn looks_printable(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let total = s.chars().count();
    let ok = s
        .chars()
        .filter(|&c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .count();
    ok * 100 >= total * 90
}

struct Crack;
impl Node for Crack {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let node_id = pstr(p, "node", "").trim();
        if node_id.is_empty() {
            return Err(CoreError::Parse(
                "请在参数里填目标节点 id（如 cloacked_pixel_extract）。".into(),
            ));
        }
        if node_id == "password_crack" {
            return Err(CoreError::Parse("不能爆破自己。".into()));
        }
        if ctx.depth > 6 {
            return Err(CoreError::Other("爆破嵌套过深。".into()));
        }
        let entry = ctx
            .registry
            .get(node_id)
            .ok_or_else(|| CoreError::Parse(format!("未知目标节点: {node_id}")))?;
        let target = entry.descriptor.clone();

        // 目标的默认参数 + extraParams 覆盖，作为每次运行的基础参数。
        let mut base = Map::new();
        for spec in &target.params {
            base.insert(spec.name.clone(), spec.default.clone());
        }
        let extra = pstr(p, "extraParams", "");
        if !extra.trim().is_empty() {
            if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(extra) {
                for (k, v) in m {
                    base.insert(k, v);
                }
            } else {
                return Err(CoreError::Parse(
                    "extraParams 不是合法的 JSON 对象。".into(),
                ));
            }
        }

        // data 转发到的输入端口：参数指定，或目标首个必填输入。
        let input_port = {
            let ip = pstr(p, "inputPort", "");
            if !ip.is_empty() {
                ip.to_string()
            } else {
                target
                    .inputs
                    .iter()
                    .find(|s| s.required)
                    .or_else(|| target.inputs.first())
                    .map(|s| s.name.clone())
                    .unwrap_or_default()
            }
        };
        let pw_param = pstr(p, "passwordParam", "password").to_string();
        let success = pstr(p, "success", "无报错(能解出)");
        let check_port = pstr(p, "checkPort", "");
        let re = if success == "正则命中" {
            Some(
                Regex::new(pstr(p, "pattern", "flag\\{"))
                    .map_err(|e| CoreError::Parse(format!("正则无效: {e}")))?,
            )
        } else {
            None
        };

        let words = in_list(i, "wordlist")?;
        let data = i.get("data").cloned();

        let mut hit: Option<(String, PortMap)> = None;
        for word in &words {
            ctx.check_cancel()?;
            let mut inputs = PortMap::new();
            if let (Some(d), false) = (&data, input_port.is_empty()) {
                inputs.insert(input_port.clone(), d.clone());
            }
            let mut params = base.clone();
            params.insert(pw_param.clone(), json!(word));
            let out = GraphExecutor::run_node_with_env(
                ctx.registry,
                node_id,
                &inputs,
                &Value::Object(params),
                ctx.env,
                &NullSink,
                ctx.cancel,
            );
            match out {
                Ok(o) => {
                    let ok = match success {
                        "正则命中" => {
                            re.as_ref().unwrap().is_match(&checked_text(&o, check_port))
                        }
                        "可打印文本" => looks_printable(&checked_text(&o, check_port)),
                        _ => true, // 无报错 → 命中
                    };
                    if ok {
                        hit = Some((word.clone(), o));
                        break;
                    }
                }
                Err(_) => { /* 口令错误：目标报错，跳过 */ }
            }
        }

        let mut m = PortMap::new();
        match hit {
            Some((word, o)) => {
                let text = o
                    .get("text")
                    .map(pv_to_string)
                    .unwrap_or_else(|| checked_text(&o, check_port));
                let bytes = match o.get("bytes") {
                    Some(PortValue::Bytes(b)) => b.clone(),
                    _ => Arc::from(text.clone().into_bytes().into_boxed_slice()),
                };
                m.insert("password".into(), PortValue::Text(word.clone()));
                m.insert("text".into(), PortValue::Text(text));
                m.insert("bytes".into(), PortValue::Bytes(bytes));
                m.insert("found".into(), PortValue::Bool(true));
                m.insert(
                    "report".into(),
                    PortValue::Text(format!(
                        "命中！口令 = \"{word}\"（目标 {node_id}，试了 {} 个候选）。",
                        words.len()
                    )),
                );
            }
            None => {
                m.insert("password".into(), PortValue::Text(String::new()));
                m.insert("text".into(), PortValue::Text(String::new()));
                m.insert("found".into(), PortValue::Bool(false));
                m.insert(
                    "report".into(),
                    PortValue::Text(format!(
                        "字典 {} 个候选均未命中（目标 {node_id}，判据「{success}」）。",
                        words.len()
                    )),
                );
            }
        }
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "password_crack",
            UTIL,
            "通用口令爆破",
            AMBER,
            vec![
                req("data", "目标输入", PortType::Any),
                req("wordlist", "字典", PortType::Any),
            ],
            vec![
                req("password", "命中口令", PortType::Text),
                opt("text", "解出文本", PortType::Text),
                opt("bytes", "解出字节", PortType::Bytes),
                opt("found", "命中", PortType::Bool),
                opt("report", "信息", PortType::Text),
            ],
            vec![
                ParamSpec::text("node", "目标节点 id", "cloacked_pixel_extract", false),
                ParamSpec::text("passwordParam", "口令参数名", "password", false),
                ParamSpec::select(
                    "success",
                    "成功判据",
                    &["无报错(能解出)", "正则命中", "可打印文本"],
                    "无报错(能解出)",
                ),
                ParamSpec::text("pattern", "正则(正则命中判据)", "flag\\{", false),
                ParamSpec::text("checkPort", "检查的输出端口(留空自动)", "", false),
                ParamSpec::text("inputPort", "目标输入端口(留空自动)", "", false),
                ParamSpec::text("extraParams", "目标额外参数(JSON)", "", false),
            ],
        )
        .heavy(),
        Arc::new(|| Arc::new(Crack)),
    );
}
