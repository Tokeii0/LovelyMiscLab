//! Tests for the encoding/crypto/text node pack, plus a full solve-chain.

use std::collections::HashMap;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::model::{Edge, NodeInstance, PortRef, SerializedGraph};
use misclab_core::graph::port::PortValue;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::json;

fn run1(descriptor: &str, text: &str, params: serde_json::Value) -> HashMap<String, PortValue> {
    let reg = default_registry();
    let mut inputs = HashMap::new();
    inputs.insert("text".to_string(), PortValue::Text(text.to_string()));
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

fn text_of(m: &HashMap<String, PortValue>, port: &str) -> String {
    match m.get(port) {
        Some(PortValue::Text(s)) => s.clone(),
        other => panic!("expected Text at '{port}', got {other:?}"),
    }
}

#[test]
fn base64_roundtrip() {
    let encoded = text_of(&run1("base64_encode", "flag{hi}", json!({})), "text");
    let decoded = text_of(&run1("base64_decode", &encoded, json!({})), "text");
    assert_eq!(decoded, "flag{hi}");
}

#[test]
fn hex_roundtrip() {
    assert_eq!(text_of(&run1("hex_encode", "AB", json!({})), "text"), "4142");
    assert_eq!(text_of(&run1("hex_decode", "4142", json!({})), "text"), "AB");
}

#[test]
fn xor_is_involution() {
    let key = json!({ "key": "k3y" });
    let once = text_of(&run1("xor", "secret message", key.clone()), "text");
    let twice = text_of(&run1("xor", &once, key), "text");
    assert_eq!(twice, "secret message");
}

#[test]
fn rot13_is_involution() {
    assert_eq!(text_of(&run1("rot13", "Hello", json!({})), "text"), "Uryyb");
    assert_eq!(text_of(&run1("rot13", "Uryyb", json!({})), "text"), "Hello");
}

#[test]
fn url_decode_works() {
    let out = run1("url_decode", "a%20b%2Bc", json!({}));
    assert_eq!(text_of(&out, "text"), "a b+c");
}

#[test]
fn regex_extract_finds_flag() {
    let out = run1(
        "regex_extract",
        "noise flag{found_it} trailing",
        json!({ "pattern": r"flag\{[^}]*\}" }),
    );
    assert_eq!(text_of(&out, "text"), "flag{found_it}");
    match out.get("matches") {
        Some(PortValue::StringList(v)) => assert_eq!(v, &vec!["flag{found_it}".to_string()]),
        other => panic!("expected StringList, got {other:?}"),
    }
}

#[test]
fn text_score_detects_flag_and_readability() {
    let out = run1("text_score", "here is flag{scored_ok}", json!({}));
    assert_eq!(text_of(&out, "flag"), "flag{scored_ok}");
    match out.get("score") {
        Some(PortValue::Number(n)) => assert!(*n > 0.9, "score {n} should be high"),
        other => panic!("expected Number, got {other:?}"),
    }
}

#[test]
fn full_solve_chain_base64_to_flag() {
    // Build the input by encoding a known flag, so the test is self-consistent.
    let secret = "flag{misc_flow_is_fun}";
    let encoded = text_of(&run1("base64_encode", secret, json!({})), "text");

    // text_input(base64) -> base64_decode -> regex_extract(flag)
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![
            NodeInstance {
                id: "in".into(),
                descriptor_id: "text_input".into(),
                params: json!({ "text": encoded }),
                position: (0.0, 0.0),
            },
            NodeInstance {
                id: "b64".into(),
                descriptor_id: "base64_decode".into(),
                params: json!({}),
                position: (0.0, 0.0),
            },
            NodeInstance {
                id: "rx".into(),
                descriptor_id: "regex_extract".into(),
                params: json!({ "pattern": r"flag\{[^}]*\}" }),
                position: (0.0, 0.0),
            },
        ],
        edges: vec![
            Edge {
                from: PortRef { node: "in".into(), port: "text".into() },
                to: PortRef { node: "b64".into(), port: "text".into() },
            },
            Edge {
                from: PortRef { node: "b64".into(), port: "text".into() },
                to: PortRef { node: "rx".into(), port: "text".into() },
            },
        ],
    };

    let exec = GraphExecutor::new(&reg, &graph).unwrap();
    let outputs = exec.run(&NullSink, &CancellationToken::new()).unwrap();
    match outputs.get("rx").and_then(|m| m.get("text")) {
        Some(PortValue::Text(s)) => assert_eq!(s, secret),
        other => panic!("chain did not recover flag: {other:?}"),
    }
}
