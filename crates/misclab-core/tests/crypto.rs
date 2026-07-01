//! Tests for hashes, radix, charset and cipher nodes. Vectors come from the
//! relevant RFCs / NIST SP 800-38A / well-known references.

use std::collections::HashMap;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::port::PortValue;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::{json, Value};

fn run_in(descriptor: &str, port: &str, text: &str, params: Value) -> HashMap<String, PortValue> {
    let reg = default_registry();
    let mut inputs = HashMap::new();
    inputs.insert(port.to_string(), PortValue::Text(text.to_string()));
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

fn hash(algo: &str, text: &str) -> String {
    text_of(&run_in("hash", "data", text, json!({ "algorithm": algo })), "text")
}

// ---- hashes ----------------------------------------------------------------

#[test]
fn hash_known_vectors() {
    assert_eq!(hash("MD5", "abc"), "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(hash("SHA1", "abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert_eq!(
        hash("SHA256", "abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(hash("MD5", ""), "d41d8cd98f00b204e9800998ecf8427e");
    // CRC-32 of "123456789" is the canonical 0xcbf43926.
    assert_eq!(hash("CRC32", "123456789"), "cbf43926");
}

#[test]
fn hmac_sha256_rfc_vector() {
    let out = text_of(
        &run_in(
            "hmac",
            "data",
            "The quick brown fox jumps over the lazy dog",
            json!({ "algorithm": "SHA256", "key": "key", "keyFormat": "UTF8" }),
        ),
        "text",
    );
    assert_eq!(out, "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8");
}

// ---- radix -----------------------------------------------------------------

#[test]
fn radix_conversions() {
    let conv = |t: &str, from: u32, to: u32| {
        text_of(&run_in("radix_convert", "text", t, json!({ "from": from, "to": to })), "text")
    };
    assert_eq!(conv("255", 10, 16), "ff");
    assert_eq!(conv("ff", 16, 2), "11111111");
    assert_eq!(conv("1010", 2, 10), "10");
    assert_eq!(conv("deadbeef", 16, 10), "3735928559");
}

#[test]
fn binary_and_decimal_roundtrip() {
    let bin = text_of(&run_in("to_binary", "data", "AB", json!({ "delimiter": "空格" })), "text");
    assert_eq!(bin, "01000001 01000010");
    assert_eq!(text_of(&run_in("from_binary", "text", &bin, json!({})), "text"), "AB");

    let dec = text_of(&run_in("to_decimal", "data", "AB", json!({})), "text");
    assert_eq!(dec, "65 66");
    assert_eq!(text_of(&run_in("from_decimal", "text", "65 66", json!({})), "text"), "AB");
}

// ---- charset ---------------------------------------------------------------

#[test]
fn charset_gbk_roundtrip() {
    let reg = default_registry();
    // "中文" in GBK is D6D0 CEC4.
    let mut enc_in = HashMap::new();
    enc_in.insert("text".to_string(), PortValue::Text("中文".to_string()));
    let enc = GraphExecutor::run_node(
        &reg,
        "encode_text",
        &enc_in,
        &json!({ "charset": "GBK" }),
        &NullSink,
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(text_of(&enc, "hex"), "d6d0cec4");

    // Feed the raw GBK bytes back into the decoder.
    let bytes = match enc.get("bytes") {
        Some(PortValue::Bytes(b)) => b.clone(),
        other => panic!("expected Bytes, got {other:?}"),
    };
    let mut dec_in = HashMap::new();
    dec_in.insert("data".to_string(), PortValue::Bytes(bytes));
    let dec = GraphExecutor::run_node(
        &reg,
        "decode_text",
        &dec_in,
        &json!({ "charset": "GBK" }),
        &NullSink,
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(text_of(&dec, "text"), "中文");
}

// ---- ciphers ---------------------------------------------------------------

#[test]
fn classical_ciphers() {
    // Atbash
    assert_eq!(text_of(&run_in("atbash", "text", "abcXYZ", json!({})), "text"), "zyxCBA");
    // Vigenère HELLO / KEY -> RIJVS, and back
    let enc = text_of(
        &run_in("vigenere", "text", "HELLO", json!({ "operation": "加密", "key": "KEY" })),
        "text",
    );
    assert_eq!(enc, "RIJVS");
    assert_eq!(
        text_of(&run_in("vigenere", "text", "RIJVS", json!({ "operation": "解密", "key": "KEY" })), "text"),
        "HELLO"
    );
    // ROT47 is self-inverse
    let r1 = text_of(&run_in("rot47", "text", "Flag{ROT47}!", json!({})), "text");
    assert_eq!(text_of(&run_in("rot47", "text", &r1, json!({})), "text"), "Flag{ROT47}!");
    // Affine encrypt/decrypt roundtrip
    let ae = text_of(
        &run_in("affine", "text", "AFFINE", json!({ "operation": "加密", "a": 5, "b": 8 })),
        "text",
    );
    assert_eq!(
        text_of(&run_in("affine", "text", &ae, json!({ "operation": "解密", "a": 5, "b": 8 })), "text"),
        "AFFINE"
    );
}

#[test]
fn rc4_known_vector() {
    // Wikipedia RC4 test vector: Key="Key", Plaintext="Plaintext" -> BBF316E8D940AF0AD3.
    let out = text_of(
        &run_in(
            "rc4",
            "text",
            "Plaintext",
            json!({ "key": "Key", "keyFormat": "UTF8", "inputFormat": "UTF8", "outputFormat": "Hex" }),
        ),
        "text",
    );
    assert_eq!(out, "bbf316e8d940af0ad3");
}

#[test]
fn aes_cbc_nist_vector() {
    // NIST SP 800-38A F.2.1 (AES-128-CBC), first block. PKCS7 appends a padding
    // block, so the ciphertext *starts with* the reference block.
    let ct = text_of(
        &run_in(
            "aes",
            "text",
            "6bc1bee22e409f96e93d7e117393172a",
            json!({
                "operation": "加密", "mode": "CBC",
                "key": "2b7e151628aed2a6abf7158809cf4f3c", "keyFormat": "Hex",
                "iv": "000102030405060708090a0b0c0d0e0f", "ivFormat": "Hex",
                "inputFormat": "Hex", "outputFormat": "Hex"
            }),
        ),
        "text",
    );
    assert!(ct.starts_with("7649abac8119b246cee98e9b12e9197d"), "got {ct}");
}

#[test]
fn aes_ctr_nist_vector() {
    // NIST SP 800-38A F.5.1 (AES-128-CTR), first block. CTR has no padding.
    let ct = text_of(
        &run_in(
            "aes",
            "text",
            "6bc1bee22e409f96e93d7e117393172a",
            json!({
                "operation": "加密", "mode": "CTR",
                "key": "2b7e151628aed2a6abf7158809cf4f3c", "keyFormat": "Hex",
                "iv": "f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff", "ivFormat": "Hex",
                "inputFormat": "Hex", "outputFormat": "Hex"
            }),
        ),
        "text",
    );
    assert_eq!(ct, "874d6191b620e3261bef6864990db6ce");
}

#[test]
fn aes_cbc_roundtrip_utf8() {
    let ct = text_of(
        &run_in(
            "aes",
            "text",
            "flag{aes_cbc_roundtrip}",
            json!({
                "operation": "加密", "mode": "CBC",
                "key": "00112233445566778899aabbccddeeff", "keyFormat": "Hex",
                "iv": "0f0e0d0c0b0a09080706050403020100", "ivFormat": "Hex",
                "inputFormat": "UTF8", "outputFormat": "Hex"
            }),
        ),
        "text",
    );
    let pt = text_of(
        &run_in(
            "aes",
            "text",
            &ct,
            json!({
                "operation": "解密", "mode": "CBC",
                "key": "00112233445566778899aabbccddeeff", "keyFormat": "Hex",
                "iv": "0f0e0d0c0b0a09080706050403020100", "ivFormat": "Hex",
                "inputFormat": "Hex", "outputFormat": "UTF8"
            }),
        ),
        "text",
    );
    assert_eq!(pt, "flag{aes_cbc_roundtrip}");
}
