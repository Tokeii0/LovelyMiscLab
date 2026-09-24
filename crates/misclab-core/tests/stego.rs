//! Tests for the steganography node pack (zero-width / StegCloak / whitespace).

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
        &run1(
            "zero_width_encode",
            secret,
            json!({ "cover": "hello world" }),
        ),
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

#[test]
fn zero_width_base4_roundtrip() {
    let secret = "flag{base4_2bit}";
    let carrier = text_of(
        &run1(
            "zero_width_encode",
            secret,
            json!({ "cover": "hi", "scheme": "四进制" }),
        ),
        "text",
    );
    assert!(carrier.contains("hi"));
    assert_eq!(
        text_of(
            &run1("zero_width_decode", &carrier, json!({ "scheme": "四进制" })),
            "text"
        ),
        secret
    );
    // Auto-detection also recovers the base-4 payload.
    assert_eq!(
        text_of(
            &run1("zero_width_decode", &carrier, json!({ "scheme": "自动" })),
            "text"
        ),
        secret
    );
}

#[test]
fn zero_width_variation_selector_roundtrip() {
    let secret = "flag{vs_smuggle}";
    let carrier = text_of(
        &run1(
            "zero_width_encode",
            secret,
            json!({ "cover": "😀", "scheme": "变体选择符" }),
        ),
        "text",
    );
    // The visible emoji is untouched…
    assert!(carrier.contains('😀'));
    // …and both explicit and auto decode recover the secret.
    assert_eq!(
        text_of(
            &run1(
                "zero_width_decode",
                &carrier,
                json!({ "scheme": "变体选择符" })
            ),
            "text"
        ),
        secret
    );
    assert_eq!(
        text_of(
            &run1("zero_width_decode", &carrier, json!({ "scheme": "自动" })),
            "text"
        ),
        secret
    );
}

#[test]
fn zero_width_unicode_tags_roundtrip() {
    let secret = "flag{tag_smuggle}";
    let carrier = text_of(
        &run1(
            "zero_width_encode",
            secret,
            json!({ "cover": "see me", "scheme": "Unicode标签" }),
        ),
        "text",
    );
    assert!(carrier.contains("see me"));
    assert_eq!(
        text_of(
            &run1(
                "zero_width_decode",
                &carrier,
                json!({ "scheme": "Unicode标签" })
            ),
            "text"
        ),
        secret
    );
    assert_eq!(
        text_of(
            &run1("zero_width_decode", &carrier, json!({ "scheme": "自动" })),
            "text"
        ),
        secret
    );
}

// ---------------------------------------------------------------- StegCloak
// Reference streams captured from the real `stegcloak` npm tool (v1.x). `reveal`
// is deterministic given the hidden stream + password, so these pin byte-exact
// compatibility of our PBKDF2-SHA512 / AES-256-CTR / HMAC / base-4-ZWC port.

// enc=false intg=false secret="flag{sc}" pw="pw"
const SC_PLAIN: &[u32] = &[
    0x200d, 0x2064, 0x200d, 0x2061, 0x200d, 0x2061, 0x200d, 0x200c, 0x2062, 0x2061, 0x200d, 0x2062,
    0x2064, 0x200d, 0x2061, 0x200c, 0x2061, 0x200c, 0x200d, 0x200c, 0x2061, 0x200c, 0x2062, 0x200c,
    0x2061, 0x200d, 0x2062, 0x200c, 0x2061, 0x2063, 0x2061,
];
// enc=true intg=false secret="flag{sc}" pw="pw"
const SC_ENC: &[u32] = &[
    0x2063, 0x200d, 0x2061, 0x200c, 0x2061, 0x200c, 0x200c, 0x200d, 0x2061, 0x200d, 0x2062, 0x2061,
    0x200d, 0x2064, 0x200d, 0x2064, 0x2064, 0x2061, 0x2061, 0x200c, 0x2061, 0x2062, 0x200d, 0x2061,
    0x2063, 0x2063, 0x200c, 0x2062, 0x200c, 0x2062, 0x200c, 0x2063, 0x2061, 0x2061, 0x2061, 0x2063,
    0x2061, 0x2061, 0x2061, 0x2062, 0x200c, 0x2062, 0x200d, 0x2061, 0x200d, 0x2064, 0x2062, 0x2061,
    0x200c, 0x2064, 0x200d, 0x2064, 0x2061, 0x200c, 0x200c, 0x200d,
];
// enc=true intg=true secret="flag{sc}" pw="pw"
const SC_HMAC: &[u32] = &[
    0x2062, 0x200c, 0x200c, 0x2064, 0x200c, 0x2064, 0x2062, 0x2063, 0x200c, 0x2061, 0x200d, 0x200c,
    0x2062, 0x200d, 0x200c, 0x200d, 0x2062, 0x2062, 0x200c, 0x200c, 0x2062, 0x2062, 0x2064, 0x2064,
    0x2062, 0x2061, 0x2062, 0x2061, 0x200d, 0x200c, 0x200c, 0x2062, 0x200c, 0x2063, 0x2063, 0x2062,
    0x2062, 0x200d, 0x2061, 0x200d, 0x2062, 0x2061, 0x200c, 0x200c, 0x2061, 0x200c, 0x2061, 0x200c,
    0x2061, 0x2063, 0x2061, 0x200c, 0x2064, 0x2061, 0x200d, 0x2064, 0x2064, 0x200c, 0x2061, 0x200d,
    0x200c, 0x2062, 0x200c, 0x200c, 0x200c, 0x2062, 0x2061, 0x200c, 0x2061, 0x200c, 0x200d, 0x2061,
    0x200c, 0x200d, 0x200c, 0x2062, 0x2061, 0x200c, 0x2062, 0x200d, 0x2061, 0x2062, 0x2062, 0x2062,
    0x2063, 0x2062, 0x2061, 0x2062, 0x200d, 0x2062, 0x200d, 0x2062, 0x200c, 0x2061, 0x2062, 0x2063,
    0x200c, 0x2062, 0x200c, 0x2061, 0x2062, 0x2063, 0x2064, 0x2062, 0x200c, 0x2062, 0x200d, 0x2061,
    0x2062, 0x200c, 0x2063, 0x200c, 0x200d, 0x2061, 0x2062, 0x200d, 0x2062, 0x2062, 0x200d, 0x2061,
    0x200c, 0x2062, 0x2062, 0x200c, 0x2062, 0x2062, 0x200d, 0x200c, 0x2062, 0x2062, 0x200c, 0x2062,
    0x2063, 0x2061, 0x200d, 0x200c, 0x2061, 0x2062, 0x2061, 0x2062, 0x200c, 0x2061, 0x200c, 0x2061,
    0x200c, 0x2061, 0x2063, 0x2061, 0x200d, 0x2062, 0x200c, 0x200c, 0x2062, 0x200d, 0x200c, 0x2062,
    0x200c, 0x200c, 0x200d, 0x2061, 0x200d, 0x2062, 0x2061, 0x200c, 0x200c, 0x2064, 0x2061, 0x2062,
    0x2062, 0x2062, 0x200d, 0x2061, 0x200d, 0x200c, 0x2061,
];
// enc=true intg=true secret="flag{StegCloak_C0mpat!}" pw="s3cr3t"
const SC_LONG: &[u32] = &[
    0x2061, 0x2063, 0x200c, 0x2062, 0x200d, 0x200c, 0x2061, 0x2064, 0x200c, 0x2062, 0x200c, 0x2062,
    0x200d, 0x200d, 0x200d, 0x200d, 0x2061, 0x200c, 0x2064, 0x2061, 0x2063, 0x2062, 0x2061, 0x200d,
    0x200c, 0x2061, 0x2062, 0x2061, 0x2061, 0x2061, 0x2063, 0x200d, 0x2064, 0x200d, 0x2061, 0x2064,
    0x2062, 0x200c, 0x2061, 0x2061, 0x200c, 0x2061, 0x200c, 0x2062, 0x2061, 0x2061, 0x200d, 0x2062,
    0x2061, 0x2062, 0x2063, 0x200c, 0x2062, 0x200c, 0x2061, 0x2064, 0x2061, 0x2064, 0x200d, 0x200d,
    0x2061, 0x200c, 0x200d, 0x2062, 0x2061, 0x2061, 0x2061, 0x2061, 0x2061, 0x200c, 0x2064, 0x200c,
    0x200d, 0x2061, 0x2063, 0x200c, 0x2062, 0x2061, 0x200c, 0x2062, 0x2063, 0x2064, 0x200d, 0x2062,
    0x2063, 0x200d, 0x2061, 0x2061, 0x200d, 0x2062, 0x2063, 0x200d, 0x2064, 0x2063, 0x2061, 0x2063,
    0x200d, 0x2061, 0x200d, 0x200c, 0x2061, 0x2063, 0x2061, 0x2061, 0x200c, 0x200d, 0x2063, 0x2061,
    0x2061, 0x200d, 0x200d, 0x2062, 0x2061, 0x2061, 0x200d, 0x200d, 0x2061, 0x200d, 0x2063, 0x200d,
    0x2061, 0x2061, 0x200c, 0x200d, 0x200c, 0x2061, 0x200c, 0x200d, 0x2063, 0x2062, 0x200c, 0x2061,
    0x2062, 0x200d, 0x2061, 0x200d, 0x2061, 0x200d, 0x200d, 0x200c, 0x2064, 0x2061, 0x2061, 0x2061,
    0x2061, 0x2061, 0x200d, 0x200d, 0x2062, 0x2061, 0x200c, 0x2061, 0x2064, 0x200c, 0x2062, 0x2061,
    0x200d, 0x2061, 0x200d, 0x2061, 0x200d, 0x2061, 0x2063, 0x2062, 0x2063, 0x2061, 0x200c, 0x2061,
    0x200d, 0x200c, 0x2064, 0x200c, 0x2061, 0x2061, 0x200d, 0x200c, 0x2062, 0x200c, 0x200d, 0x2063,
    0x2062, 0x200c, 0x2062, 0x2061, 0x200d, 0x200c, 0x2064, 0x2063, 0x200d, 0x2061, 0x2061, 0x2064,
    0x200c, 0x200d, 0x2061, 0x200d, 0x200c, 0x2062, 0x2061, 0x2062, 0x200d, 0x2063, 0x2061, 0x2061,
    0x2064, 0x200c, 0x200d, 0x2063, 0x2061, 0x200c, 0x2061, 0x2064, 0x2064, 0x2062, 0x200c, 0x2061,
    0x2061,
];

fn from_cps(cps: &[u32]) -> String {
    cps.iter().map(|&c| char::from_u32(c).unwrap()).collect()
}

/// Reveal a real-tool vector woven between visible words.
fn sc_reveal_vec(stream: &[u32], pw: &str) -> String {
    let carrier = format!("hello {}world foo", from_cps(stream));
    text_of(
        &run1("stegcloak_reveal", &carrier, json!({ "password": pw })),
        "text",
    )
}

#[test]
fn stegcloak_reveal_real_vectors() {
    assert_eq!(sc_reveal_vec(SC_PLAIN, "pw"), "flag{sc}");
    assert_eq!(sc_reveal_vec(SC_ENC, "pw"), "flag{sc}");
    assert_eq!(sc_reveal_vec(SC_HMAC, "pw"), "flag{sc}");
    assert_eq!(sc_reveal_vec(SC_LONG, "s3cr3t"), "flag{StegCloak_C0mpat!}");
}

#[test]
fn stegcloak_reports_hmac_status() {
    let carrier = format!("hi {}there", from_cps(SC_HMAC));
    let good = run1("stegcloak_reveal", &carrier, json!({ "password": "pw" }));
    assert!(text_of(&good, "report").contains("通过"));
    let bad = run1("stegcloak_reveal", &carrier, json!({ "password": "wrong" }));
    assert!(text_of(&bad, "report").contains("失败"));
}

#[test]
fn stegcloak_roundtrip_all_modes() {
    let secret = "flag{stegcloak_roundtrip_测试}";
    for (enc, intg) in [(true, true), (true, false), (false, false)] {
        let carrier = text_of(
            &run1(
                "stegcloak_hide",
                secret,
                json!({ "cover": "look ma no ink", "password": "hunter2", "encrypt": enc, "integrity": intg }),
            ),
            "text",
        );
        assert!(carrier.contains("look"));
        assert_eq!(
            text_of(
                &run1(
                    "stegcloak_reveal",
                    &carrier,
                    json!({ "password": "hunter2" })
                ),
                "text"
            ),
            secret,
            "roundtrip failed for enc={enc} intg={intg}"
        );
    }
}

// ---------------------------------------------------------------- whitespace

#[test]
fn whitespace_roundtrip_and_scope() {
    let secret = "flag{snow}";
    let carrier = text_of(
        &run1(
            "whitespace_encode",
            secret,
            json!({ "cover": "nothing to see here" }),
        ),
        "text",
    );
    // Visible text intact; only trailing space/tab added.
    assert!(carrier.starts_with("nothing to see here"));
    assert!(carrier.contains('\t') || carrier.contains(' '));
    assert_eq!(
        text_of(&run1("whitespace_decode", &carrier, json!({})), "text"),
        secret
    );
    // "全部" scope reads every space/tab, so exercise it on a cover-less carrier.
    let bare = text_of(
        &run1("whitespace_encode", secret, json!({ "cover": "" })),
        "text",
    );
    assert_eq!(
        text_of(
            &run1("whitespace_decode", &bare, json!({ "scope": "全部" })),
            "text"
        ),
        secret
    );
}

#[test]
fn whitespace_tab_is_zero() {
    let secret = "Hi";
    let carrier = text_of(
        &run1(
            "whitespace_encode",
            secret,
            json!({ "cover": "x", "zero": "制表符 (tab)" }),
        ),
        "text",
    );
    assert_eq!(
        text_of(
            &run1(
                "whitespace_decode",
                &carrier,
                json!({ "zero": "制表符 (tab)" })
            ),
            "text"
        ),
        secret
    );
}
