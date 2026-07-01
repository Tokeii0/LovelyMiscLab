//! Base-N node tests. Known vectors are taken from CyberChef's own operation
//! descriptions / the relevant RFCs, so we stay byte-compatible with it.

use std::collections::HashMap;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::port::PortValue;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::{json, Value};

fn run_in(descriptor: &str, port: &str, val: PortValue, params: Value) -> HashMap<String, PortValue> {
    let reg = default_registry();
    let mut inputs = HashMap::new();
    inputs.insert(port.to_string(), val);
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

/// Encode: feed text on the `data` (Any) input.
fn enc(descriptor: &str, text: &str, params: Value) -> String {
    text_of(&run_in(descriptor, "data", PortValue::Text(text.to_string()), params), "text")
}

/// Decode: feed the encoded string on `text`, read the decoded text back.
fn dec(descriptor: &str, text: &str, params: Value) -> String {
    text_of(&run_in(descriptor, "text", PortValue::Text(text.to_string()), params), "text")
}

#[test]
fn base58_known_vector_and_roundtrip() {
    // From CyberChef's To Base58 description.
    assert_eq!(enc("base58_encode", "hello world", json!({})), "StV1DL6CwTryKyV");
    assert_eq!(dec("base58_decode", "StV1DL6CwTryKyV", json!({})), "hello world");
}

#[test]
fn base58_ripple_roundtrip() {
    let e = enc("base58_encode", "flag{ripple}", json!({ "variant": "Ripple" }));
    assert_eq!(dec("base58_decode", &e, json!({ "variant": "Ripple" })), "flag{ripple}");
}

#[test]
fn base85_known_vector_and_roundtrip() {
    // From CyberChef's To Base85 description.
    assert_eq!(enc("base85_encode", "hello world", json!({})), "BOu!rD]j7BEbo7");
    assert_eq!(dec("base85_decode", "BOu!rD]j7BEbo7", json!({})), "hello world");
}

#[test]
fn base85_z85_roundtrip() {
    let e = enc("base85_encode", "flag{z85_works}", json!({ "variant": "Z85" }));
    assert_eq!(dec("base85_decode", &e, json!({ "variant": "Z85" })), "flag{z85_works}");
}

#[test]
fn base85_delimiters() {
    let e = enc("base85_encode", "hi there", json!({ "delim": true }));
    assert!(e.starts_with("<~") && e.ends_with("~>"), "got {e}");
    // Decoder strips the delimiters automatically.
    assert_eq!(dec("base85_decode", &e, json!({})), "hi there");
}

#[test]
fn base45_known_vector_and_roundtrip() {
    // RFC 9285 §4.3 test vector: "AB" -> "BB8".
    assert_eq!(enc("base45_encode", "AB", json!({})), "BB8");
    assert_eq!(dec("base45_decode", "BB8", json!({})), "AB");
    // Longer roundtrip through a flag.
    let e = enc("base45_encode", "flag{base45}", json!({}));
    assert_eq!(dec("base45_decode", &e, json!({})), "flag{base45}");
}

#[test]
fn base32_roundtrip_both_alphabets() {
    let e = enc("base32_encode", "flag{base32}", json!({}));
    assert_eq!(dec("base32_decode", &e, json!({})), "flag{base32}");
    let h = enc("base32_encode", "flag{hex32}", json!({ "variant": "Hex 扩展" }));
    assert_eq!(dec("base32_decode", &h, json!({ "variant": "Hex 扩展" })), "flag{hex32}");
}

#[test]
fn base62_roundtrip() {
    let e = enc("base62_encode", "flag{base62}", json!({}));
    assert_eq!(dec("base62_decode", &e, json!({})), "flag{base62}");
}

#[test]
fn base92_roundtrip() {
    let e = enc("base92_encode", "flag{base92}", json!({}));
    assert_eq!(dec("base92_decode", &e, json!({})), "flag{base92}");
}

#[test]
fn decode_ignores_whitespace_and_junk() {
    // Non-alphabet chars are stripped by default.
    assert_eq!(dec("base58_decode", "StV1DL6C wTryKyV", json!({})), "hello world");
}
