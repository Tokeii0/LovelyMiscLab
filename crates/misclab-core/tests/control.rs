//! Tests for the control/logic nodes and the optimized encoding options.

use std::collections::HashMap;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::port::PortValue;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::json;

fn run(
    descriptor: &str,
    inputs: Vec<(&str, PortValue)>,
    params: serde_json::Value,
) -> HashMap<String, PortValue> {
    let reg = default_registry();
    let map: HashMap<String, PortValue> =
        inputs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    GraphExecutor::run_node(&reg, descriptor, &map, &params, &NullSink, &CancellationToken::new())
        .unwrap()
}

fn text(s: &str) -> PortValue {
    PortValue::Text(s.to_string())
}

#[test]
fn switch_selects_branch() {
    let yes = run(
        "switch",
        vec![("condition", PortValue::Bool(true)), ("a", text("yes")), ("b", text("no"))],
        json!({}),
    );
    assert!(matches!(yes.get("output"), Some(PortValue::Text(s)) if s == "yes"));

    let no = run(
        "switch",
        vec![("condition", PortValue::Bool(false)), ("a", text("yes")), ("b", text("no"))],
        json!({}),
    );
    assert!(matches!(no.get("output"), Some(PortValue::Text(s)) if s == "no"));
}

#[test]
fn compare_operators() {
    let eq = run("compare", vec![("a", text("abc")), ("b", text("abc"))], json!({ "op": "==" }));
    assert!(matches!(eq.get("result"), Some(PortValue::Bool(true))));

    let contains =
        run("compare", vec![("a", text("hello world")), ("b", text("world"))], json!({ "op": "包含" }));
    assert!(matches!(contains.get("result"), Some(PortValue::Bool(true))));

    let re = run(
        "compare",
        vec![("a", text("flag{x}")), ("b", text(r"flag\{.*\}"))],
        json!({ "op": "匹配正则" }),
    );
    assert!(matches!(re.get("result"), Some(PortValue::Bool(true))));
}

#[test]
fn concat_split_length() {
    let cat = run("concat", vec![("a", text("foo")), ("b", text("bar"))], json!({ "sep": "-" }));
    assert!(matches!(cat.get("text"), Some(PortValue::Text(s)) if s == "foo-bar"));

    let sp = run("split", vec![("text", text("a,b,c"))], json!({ "sep": "," }));
    match sp.get("list") {
        Some(PortValue::StringList(v)) => assert_eq!(v, &vec!["a", "b", "c"]),
        other => panic!("expected StringList, got {other:?}"),
    }

    let len = run("length", vec![("text", text("hello"))], json!({}));
    assert!(matches!(len.get("length"), Some(PortValue::Number(n)) if (*n - 5.0).abs() < 1e-9));
}

#[test]
fn base64_url_safe_variant() {
    let enc = run("base64_encode", vec![("text", text("~~~?"))], json!({ "variant": "URL安全" }));
    let encoded = match enc.get("text") {
        Some(PortValue::Text(s)) => s.clone(),
        other => panic!("expected Text, got {other:?}"),
    };
    let dec =
        run("base64_decode", vec![("text", text(&encoded))], json!({ "variant": "URL安全" }));
    assert!(matches!(dec.get("text"), Some(PortValue::Text(s)) if s == "~~~?"));
}

#[test]
fn regex_preset_md5() {
    let out = run(
        "regex_extract",
        vec![("text", text("hash: d41d8cd98f00b204e9800998ecf8427e end"))],
        json!({ "preset": "MD5" }),
    );
    assert!(matches!(out.get("text"), Some(PortValue::Text(s)) if s == "d41d8cd98f00b204e9800998ecf8427e"));
}

#[test]
fn xor_bruteforce_recovers_flag() {
    // XOR an ASCII flag with 0x42 (result stays ASCII), then brute-force it back.
    let scrambled: String = "flag{ok}".bytes().map(|b| (b ^ 0x42) as char).collect();
    let out = run("xor_bruteforce", vec![("text", text(&scrambled))], json!({}));
    match out.get("candidates") {
        Some(PortValue::Candidates(cands)) => {
            assert!(cands.iter().any(|c| c.text == "flag{ok}"), "candidates: {cands:?}");
        }
        other => panic!("expected Candidates, got {other:?}"),
    }
}

fn text_out(m: &HashMap<String, PortValue>, port: &str) -> String {
    match m.get(port) {
        Some(PortValue::Text(s)) => s.clone(),
        other => panic!("expected Text at '{port}', got {other:?}"),
    }
}

#[test]
fn loop_decode_unwraps_nested_base64() {
    // Encode a flag twice, then loop-decode until the flag pattern appears.
    let once = text_out(&run("base64_encode", vec![("text", text("flag{loop}"))], json!({})), "text");
    let twice = text_out(&run("base64_encode", vec![("text", text(&once))], json!({})), "text");

    let out = run(
        "loop_decode",
        vec![("text", text(&twice))],
        json!({ "codec": "Base64", "until": "匹配正则", "pattern": r"flag\{[^}]*\}", "max": 16 }),
    );
    assert_eq!(text_out(&out, "text"), "flag{loop}");
    assert!(matches!(out.get("hit"), Some(PortValue::Bool(true))));
}

#[test]
fn magic_decode_finds_flag_chain() {
    let encoded = text_out(&run("base64_encode", vec![("text", text("flag{magic}"))], json!({})), "text");
    let out = run("magic_decode", vec![("text", text(&encoded))], json!({}));
    assert_eq!(text_out(&out, "text"), "flag{magic}");
    assert!(matches!(out.get("hit"), Some(PortValue::Bool(true))));
}
