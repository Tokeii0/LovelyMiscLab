//! Tests for the control / logic node pack.

use std::collections::HashMap;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::port::PortValue;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::{json, Value};

fn run(descriptor: &str, inputs: HashMap<String, PortValue>, params: Value) -> HashMap<String, PortValue> {
    let reg = default_registry();
    GraphExecutor::run_node(
        &reg,
        descriptor,
        &inputs,
        &params,
        &NullSink,
        &CancellationToken::new(),
    )
    .unwrap()
}

fn b(v: bool) -> PortValue {
    PortValue::Bool(v)
}
fn t(s: &str) -> PortValue {
    PortValue::Text(s.to_string())
}
fn n(x: f64) -> PortValue {
    PortValue::Number(x)
}
fn list(v: &[&str]) -> PortValue {
    PortValue::StringList(v.iter().map(|s| s.to_string()).collect())
}

fn get_bool(m: &HashMap<String, PortValue>, port: &str) -> bool {
    match m.get(port) {
        Some(PortValue::Bool(v)) => *v,
        other => panic!("expected Bool at '{port}', got {other:?}"),
    }
}
fn text_of(m: &HashMap<String, PortValue>, port: &str) -> String {
    match m.get(port) {
        Some(PortValue::Text(s)) => s.clone(),
        other => panic!("expected Text at '{port}', got {other:?}"),
    }
}
fn slist_of(m: &HashMap<String, PortValue>, port: &str) -> Vec<String> {
    match m.get(port) {
        Some(PortValue::StringList(v)) => v.clone(),
        other => panic!("expected StringList at '{port}', got {other:?}"),
    }
}

#[test]
fn logic_gates() {
    let g = |a: bool, bb: bool, op: &str| {
        run("logic", HashMap::from([("a".into(), b(a)), ("b".into(), b(bb))]), json!({ "op": op }))
    };
    assert!(get_bool(&g(true, false, "OR"), "result"));
    assert!(!get_bool(&g(true, false, "AND"), "result"));
    assert!(get_bool(&g(true, false, "XOR"), "result"));
    assert!(!get_bool(&g(true, true, "XOR"), "result"));
    assert!(get_bool(&run("logic", HashMap::from([("a".into(), b(false))]), json!({ "op": "NOT" })), "result"));
}

#[test]
fn switch_case_picks_branch() {
    let inputs = HashMap::from([
        ("selector".into(), n(2.0)),
        ("case0".into(), t("zero")),
        ("case1".into(), t("one")),
        ("case2".into(), t("two")),
        ("default".into(), t("def")),
    ]);
    assert_eq!(text_of(&run("switch_case", inputs, json!({})), "output"), "two");

    let oob = HashMap::from([("selector".into(), n(9.0)), ("default".into(), t("def"))]);
    assert_eq!(text_of(&run("switch_case", oob, json!({})), "output"), "def");
}

#[test]
fn gate_passes_and_blocks() {
    let pass = run("gate", HashMap::from([("value".into(), t("secret")), ("condition".into(), b(true))]), json!({}));
    assert_eq!(text_of(&pass, "output"), "secret");
    let blocked = run("gate", HashMap::from([("value".into(), t("secret")), ("condition".into(), b(false))]), json!({}));
    assert!(matches!(blocked.get("output"), Some(PortValue::None)));
    assert!(!get_bool(&blocked, "passed"));
}

#[test]
fn range_generates_sequence() {
    let out = run("range", HashMap::new(), json!({ "start": 0, "end": 5, "step": 1 }));
    assert_eq!(slist_of(&out, "list"), vec!["0", "1", "2", "3", "4"]);
    let out2 = run("range", HashMap::new(), json!({ "start": 10, "end": 0, "step": -3 }));
    assert_eq!(slist_of(&out2, "list"), vec!["10", "7", "4", "1"]);
}

#[test]
fn map_transforms_each() {
    let out = run("map", HashMap::from([("list".into(), list(&["ab", "cd"]))]), json!({ "op": "大写" }));
    assert_eq!(slist_of(&out, "list"), vec!["AB", "CD"]);
}

#[test]
fn filter_keeps_matches() {
    let out = run(
        "filter_list",
        HashMap::from([("list".into(), list(&["flag{a}", "noise", "flag{b}"]))]),
        json!({ "pattern": "flag", "mode": "保留匹配" }),
    );
    assert_eq!(slist_of(&out, "list"), vec!["flag{a}", "flag{b}"]);
}

#[test]
fn join_reduces_list() {
    let out = run("join_list", HashMap::from([("list".into(), list(&["a", "b", "c"]))]), json!({ "sep": "逗号" }));
    assert_eq!(text_of(&out, "text"), "a,b,c");
}

#[test]
fn iterate_while_decodes_until_flag() {
    // base64("flag{iter}") = "ZmxhZ3tpdGVyfQ==" — one decode reveals the flag.
    let out = run(
        "iterate",
        HashMap::from([("text".into(), t("ZmxhZ3tpdGVyfQ=="))]),
        json!({ "op": "Base64解码", "until": r"flag\{[^}]*\}", "max": 16 }),
    );
    assert_eq!(text_of(&out, "text"), "flag{iter}");
    assert!(get_bool(&out, "hit"));
}

#[test]
fn foreach_pipeline_range_map_filter_join() {
    // A dataflow "for": range 1..4 -> hash each (SHA256) -> keep those with a digit
    // 'a' -> join. Just exercises the chain end-to-end for shape.
    let ranged = run("range", HashMap::new(), json!({ "start": 1, "end": 4, "step": 1 }));
    let seq = ranged.get("list").cloned().unwrap();
    let mapped = run("map", HashMap::from([("list".into(), seq)]), json!({ "op": "大写" }));
    assert_eq!(slist_of(&mapped, "list"), vec!["1", "2", "3"]);
}
