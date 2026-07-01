//! Tests for the steganography node pack (zero-width characters).

use std::collections::HashMap;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
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
fn zero_width_roundtrip_auto() {
    let secret = "flag{zero_width_ftw}";
    let carrier = text_of(
        &run1("zero_width_encode", secret, json!({ "cover": "hello world" })),
        "text",
    );
    // The cover text is still visible…
    assert!(carrier.contains("hello world"));
    // …and zero-width symbols are woven in.
    assert!(carrier.chars().any(|c| c == '\u{200B}' || c == '\u{200C}'));
    // Auto-detection recovers the secret without being told the mapping.
    let decoded = text_of(
        &run1("zero_width_decode", &carrier, json!({ "scheme": "自动" })),
        "text",
    );
    assert_eq!(decoded, secret);
}

#[test]
fn zero_width_roundtrip_explicit_mapping() {
    let secret = "hidden";
    let carrier = text_of(
        &run1(
            "zero_width_encode",
            secret,
            json!({ "cover": "", "zero": "ZWNJ (U+200C)", "one": "ZWJ (U+200D)" }),
        ),
        "text",
    );
    let decoded = text_of(
        &run1(
            "zero_width_decode",
            &carrier,
            json!({ "scheme": "二进制", "zero": "ZWNJ (U+200C)", "one": "ZWJ (U+200D)" }),
        ),
        "text",
    );
    assert_eq!(decoded, secret);
}

#[test]
fn zero_width_decode_known_vector() {
    // 'A' = 0x41 = 0100_0001 (MSB). 0 -> ZWSP, 1 -> ZWNJ. Wrapped in normal text.
    let bits = "01000001";
    let hidden: String = bits
        .chars()
        .map(|b| if b == '1' { '\u{200C}' } else { '\u{200B}' })
        .collect();
    let carrier = format!("x{hidden}y");
    let decoded = text_of(
        &run1("zero_width_decode", &carrier, json!({ "scheme": "二进制" })),
        "text",
    );
    assert_eq!(decoded, "A");
}

#[test]
fn zero_width_decode_reports_when_absent() {
    let out = run1("zero_width_decode", "just plain text", json!({}));
    assert_eq!(text_of(&out, "text"), "");
    assert!(text_of(&out, "report").contains("未发现"));
}
