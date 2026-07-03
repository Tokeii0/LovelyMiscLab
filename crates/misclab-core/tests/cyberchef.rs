//! Tests for the CyberChef-parity node pack (text / classical cipher / format).

use std::collections::HashMap;
use std::sync::Arc;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::port::PortValue;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::{json, Value};

fn run(descriptor: &str, port: &str, text: &str, params: Value) -> HashMap<String, PortValue> {
    let reg = default_registry();
    let mut inputs = HashMap::new();
    inputs.insert(port.to_string(), PortValue::Text(text.to_string()));
    GraphExecutor::run_node(&reg, descriptor, &inputs, &params, &NullSink, &CancellationToken::new()).unwrap()
}
fn text_of(m: &HashMap<String, PortValue>, port: &str) -> String {
    match m.get(port) {
        Some(PortValue::Text(s)) => s.clone(),
        o => panic!("expected Text at '{port}', got {o:?}"),
    }
}
fn t(descriptor: &str, text: &str, params: Value) -> String {
    text_of(&run(descriptor, "text", text, params), "text")
}
fn hx(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}
fn run_bytes(descriptor: &str, port: &str, data: Vec<u8>, params: Value) -> HashMap<String, PortValue> {
    let reg = default_registry();
    let mut inputs = HashMap::new();
    inputs.insert(port.to_string(), PortValue::Bytes(Arc::from(data.into_boxed_slice())));
    GraphExecutor::run_node(&reg, descriptor, &inputs, &params, &NullSink, &CancellationToken::new()).unwrap()
}

#[test]
fn caesar_shift() {
    assert_eq!(t("caesar", "HELLO", json!({ "amount": 3 })), "KHOOR");
}

#[test]
fn morse_roundtrip() {
    assert_eq!(t("morse_encode", "SOS", json!({})), "... --- ...");
    assert_eq!(t("morse_decode", "... --- ...", json!({})), "SOS");
}

#[test]
fn bacon_roundtrip() {
    assert_eq!(t("bacon_encode", "AB", json!({})), "AAAAAAAAAB");
    assert_eq!(t("bacon_decode", "AAAAAAAAAB", json!({})), "AB");
}

#[test]
fn a1z26_roundtrip() {
    assert_eq!(t("a1z26_encode", "ABC", json!({})), "1 2 3");
    assert_eq!(t("a1z26_decode", "1 2 3", json!({})), "ABC");
}

#[test]
fn change_case_ops() {
    assert_eq!(t("change_case", "abc", json!({ "mode": "大写" })), "ABC");
    assert_eq!(t("change_case", "AbC", json!({ "mode": "交换大小写" })), "aBc");
    assert_eq!(t("change_case", "hello world", json!({ "mode": "词首大写" })), "Hello World");
}

#[test]
fn rail_fence_roundtrip() {
    let e = t("rail_fence_encode", "WEAREDISCOVEREDFLEEATONCE", json!({ "rails": 3 }));
    assert_eq!(e, "WECRLTEERDSOEEFEAOCAIVDEN");
    assert_eq!(t("rail_fence_decode", &e, json!({ "rails": 3 })), "WEAREDISCOVEREDFLEEATONCE");
}

#[test]
fn html_entity_roundtrip() {
    assert_eq!(
        t("html_entity_encode", "<a href=\"x\">", json!({})),
        "&lt;a href=&quot;x&quot;&gt;"
    );
    assert_eq!(t("html_entity_decode", "&lt;a&gt;&amp;&#65;", json!({})), "<a>&A");
}

#[test]
fn unicode_escape_roundtrip() {
    assert_eq!(t("unicode_escape", "中", json!({})), "\\u4e2d");
    assert_eq!(t("unicode_unescape", "\\u4e2d", json!({})), "中");
}

#[test]
fn octal_roundtrip() {
    let e = text_of(&run("to_octal", "data", "AB", json!({})), "text");
    assert_eq!(e, "101 102");
    assert_eq!(t("from_octal", &e, json!({})), "AB");
}

#[test]
fn hexdump_roundtrip() {
    let dump = text_of(&run("to_hexdump", "data", "hello world", json!({})), "text");
    assert!(dump.contains("68 65 6c 6c 6f"));
    assert_eq!(t("from_hexdump", &dump, json!({})), "hello world");
}

#[test]
fn entropy_of_uniform_is_zero() {
    match run("entropy", "data", "aaaa", json!({})).get("entropy") {
        Some(PortValue::Number(n)) => assert!(n.abs() < 0.001, "got {n}"),
        o => panic!("expected Number, got {o:?}"),
    }
}

#[test]
fn sort_and_unique() {
    assert_eq!(t("sort_lines", "b\na\nc", json!({ "order": "字母升序" })), "a\nb\nc");
    assert_eq!(t("unique_lines", "a\nb\na\nc\nb", json!({})), "a\nb\nc");
}

#[test]
fn defang_refang() {
    let d = t("defang", "http://evil.com/x", json!({ "operation": "defang" }));
    assert_eq!(d, "hxxp[://]evil[.]com/x");
    assert_eq!(t("defang", &d, json!({ "operation": "refang" })), "http://evil.com/x");
}

#[test]
fn des_and_3des_roundtrip() {
    let enc = |op: &str, input: &str, key: &str, infmt: &str, outfmt: &str| {
        t(
            "des",
            input,
            json!({
                "operation": op, "mode": "CBC", "key": key, "keyFormat": "Hex",
                "iv": "0000000000000000", "ivFormat": "Hex",
                "inputFormat": infmt, "outputFormat": outfmt
            }),
        )
    };
    let ct = enc("加密", "hello123", "0123456789abcdef", "UTF8", "Hex");
    assert_eq!(enc("解密", &ct, "0123456789abcdef", "Hex", "UTF8"), "hello123");
    // 3DES (24-byte key)
    let k3 = "0123456789abcdef0123456789abcdef0123456789abcdef";
    let ct3 = enc("加密", "flag{3des}", k3, "UTF8", "Hex");
    assert_eq!(enc("解密", &ct3, k3, "Hex", "UTF8"), "flag{3des}");
}

#[test]
fn blowfish_roundtrip() {
    let go = |op: &str, input: &str, infmt: &str, outfmt: &str| {
        t(
            "blowfish",
            input,
            json!({
                "operation": op, "mode": "CBC", "key": "0011223344556677", "keyFormat": "Hex",
                "iv": "0000000000000000", "ivFormat": "Hex",
                "inputFormat": infmt, "outputFormat": outfmt
            }),
        )
    };
    let ct = go("加密", "flag{blowfish}", "UTF8", "Hex");
    assert_eq!(go("解密", &ct, "Hex", "UTF8"), "flag{blowfish}");
}

#[test]
fn chacha20_rfc7539_keystream() {
    let zeros64 = "00".repeat(64);
    let key = "00".repeat(32);
    let nonce = "00".repeat(12);
    let ct = t(
        "chacha20",
        &zeros64,
        json!({
            "variant": "ChaCha20", "key": key, "keyFormat": "Hex",
            "nonce": nonce, "nonceFormat": "Hex", "inputFormat": "Hex", "outputFormat": "Hex"
        }),
    );
    assert!(ct.starts_with("76b8e0ada0f13d90"), "got {ct}");
}

#[test]
fn salsa20_roundtrip() {
    let go = |input: &str, infmt: &str, outfmt: &str| {
        t(
            "salsa20",
            input,
            json!({
                "key": "00".repeat(32), "keyFormat": "Hex",
                "nonce": "0001020304050607", "nonceFormat": "Hex",
                "inputFormat": infmt, "outputFormat": outfmt
            }),
        )
    };
    let ct = go("flag{salsa}", "UTF8", "Hex");
    assert_eq!(go(&ct, "Hex", "UTF8"), "flag{salsa}");
}

#[test]
fn blake2b_and_whirlpool() {
    let h = |algo: &str, input: &str| {
        text_of(&run("hash", "data", input, json!({ "algorithm": algo })), "text")
    };
    assert_eq!(
        h("BLAKE2b", "abc"),
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
    );
    // Whirlpool of empty string.
    assert_eq!(
        h("Whirlpool", ""),
        "19fa61d75522a4669b44e39c1d2e1726c530232130d407f89afee0964997f7a73e83be698b288febcf88e3e03c4f0757ea8964e59b63d93708b138cc42a66eb3"
    );
}

#[test]
fn gzip_roundtrip() {
    // compress (Gzip) → archive_extract (gz) — both kept after decompress was removed.
    let reg = default_registry();
    let mut i = HashMap::new();
    i.insert("data".to_string(), PortValue::Text("hello world hello world".to_string()));
    let comp = GraphExecutor::run_node(&reg, "compress", &i, &json!({ "format": "Gzip" }), &NullSink, &CancellationToken::new()).unwrap();
    let bytes = match comp.get("bytes") {
        Some(PortValue::Bytes(b)) => b.clone(),
        o => panic!("expected Bytes, got {o:?}"),
    };
    let mut i2 = HashMap::new();
    i2.insert("archive".to_string(), PortValue::Bytes(bytes));
    let dec = GraphExecutor::run_node(&reg, "archive_extract", &i2, &json!({}), &NullSink, &CancellationToken::new()).unwrap();
    assert_eq!(text_of(&dec, "text"), "hello world hello world");
}

#[test]
fn json_substitution_braille() {
    assert_eq!(
        t("json_format", "{ \"a\": 1, \"b\": [2,3] }", json!({ "operation": "压缩" })),
        "{\"a\":1,\"b\":[2,3]}"
    );
    let rev: String = ('A'..='Z').rev().collect();
    assert_eq!(
        t("substitution", "ABCXYZ", json!({ "from": "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "to": rev })),
        "ZYXCBA"
    );
    assert_eq!(t("braille_encode", "abc", json!({})), "⠁⠃⠉");
    assert_eq!(t("braille_decode", "⠁⠃⠉", json!({})), "abc");
}

#[test]
fn rsa_recovery_and_decrypt() {
    // Wikipedia example: p=61, q=53, e=17 → n=3233, d=2753; 65 ↔ 2790.
    let params = run("rsa_params", "text", "", json!({ "p": "61", "q": "53", "e": "17" }));
    assert_eq!(text_of(&params, "n"), "3233");
    assert_eq!(text_of(&params, "d"), "2753");
    let dec = run("rsa_decrypt", "text", "2790", json!({ "n": "3233", "d": "2753" }));
    assert_eq!(text_of(&dec, "int"), "65");
    assert_eq!(text_of(&dec, "text"), "A");
    // Deriving d from p,q,e:
    let dec2 = run("rsa_decrypt", "text", "2790", json!({ "p": "61", "q": "53", "e": "17" }));
    assert_eq!(text_of(&dec2, "int"), "65");
}

#[test]
fn bifid_and_playfair() {
    let e = t("bifid_encode", "ATTACKATDAWN", json!({ "keyword": "CIPHER" }));
    assert_eq!(t("bifid_decode", &e, json!({ "keyword": "CIPHER" })), "ATTACKATDAWN");
    // Playfair MONARCHY: HI -> BF (rectangle rule).
    assert_eq!(t("playfair_encode", "HI", json!({ "keyword": "MONARCHY" })), "BF");
    let pe = t("playfair_encode", "HIDEGOLD", json!({ "keyword": "PLAYFAIREXAMPLE" }));
    assert_eq!(t("playfair_decode", &pe, json!({ "keyword": "PLAYFAIREXAMPLE" })), "HIDEGOLD");
}

#[test]
fn filetype_detect() {
    assert!(text_of(&run("detect_file_type", "data", "%PDF-1.7 body", json!({})), "type").contains("PDF"));
    assert!(text_of(&run("detect_file_type", "data", "RIFF????WEBP data", json!({})), "type").contains("WEBP"));
}

#[test]
fn extract_tokens() {
    let ip = run("extract", "text", "srv 10.0.0.1 x 8.8.8.8 10.0.0.1", json!({ "kind": "IPv4" }));
    assert_eq!(text_of(&ip, "text"), "10.0.0.1\n8.8.8.8"); // deduped, in order
    let mail = run("extract", "text", "a@b.com c@d.org", json!({ "kind": "邮箱" }));
    assert_eq!(text_of(&mail, "text"), "a@b.com\nc@d.org");
}

#[test]
fn charcode_roundtrip() {
    assert_eq!(t("to_charcode", "AB", json!({ "base": "16", "delimiter": "空格" })), "41 42");
    assert_eq!(t("from_charcode", "41 42", json!({ "base": "16" })), "AB");
    assert_eq!(t("to_charcode", "AB", json!({ "base": "10" })), "65 66");
}

#[test]
fn quoted_printable_roundtrip() {
    assert_eq!(text_of(&run("quoted_printable_encode", "data", "café", json!({})), "text"), "caf=C3=A9");
    assert_eq!(t("quoted_printable_decode", "caf=C3=A9", json!({})), "café");
}

#[test]
fn rotate_and_adler() {
    // 'A' = 0x41 = 0b01000001
    assert_eq!(text_of(&run("rotate_bytes", "data", "A", json!({ "direction": "左(ROL)", "amount": 1 })), "hex"), "82");
    assert_eq!(text_of(&run("rotate_bytes", "data", "A", json!({ "direction": "右(ROR)", "amount": 1 })), "hex"), "a0");
    // Adler-32("Wikipedia") = 0x11E60398 (Wikipedia's own example)
    assert_eq!(text_of(&run("hash", "data", "Wikipedia", json!({ "algorithm": "Adler-32" })), "text"), "11e60398");
}

#[test]
fn sm3_vector() {
    // GM/T 0004-2012 test vector
    assert_eq!(
        text_of(&run("hash", "data", "abc", json!({ "algorithm": "SM3" })), "text"),
        "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
    );
}

#[test]
fn bcrypt_roundtrip() {
    let h = t("bcrypt", "hunter2", json!({ "operation": "哈希", "cost": 4 }));
    assert!(h.starts_with("$2"));
    let v = run("bcrypt", "text", "hunter2", json!({ "operation": "校验", "hash": h }));
    assert_eq!(text_of(&v, "text"), "匹配 ✓");
    let bad = run("bcrypt", "text", "wrong", json!({ "operation": "校验", "hash": h }));
    assert_eq!(text_of(&bad, "text"), "不匹配 ✗");
}

#[test]
fn enigma_known_vector() {
    // Rotors I II III, reflector B, rings AAA, ground AAA, no plugboard: AAAAA -> BDZGO
    let cfg = json!({ "rotors": "I II III", "reflector": "B", "ring": "AAA", "position": "AAA" });
    assert_eq!(t("enigma", "AAAAA", cfg.clone()), "BDZGO");
    assert_eq!(t("enigma", "BDZGO", cfg), "AAAAA"); // symmetric
}

#[test]
fn adfgvx_roundtrip() {
    let enc = t("adfgvx", "ATTACKAT1200AM", json!({ "operation": "加密", "keyword": "PRIVACY" }));
    let dec = t("adfgvx", &enc, json!({ "operation": "解密", "keyword": "PRIVACY" }));
    assert_eq!(dec, "ATTACKAT1200AM");
}

#[test]
fn exif_extract_make() {
    // minimal little-endian TIFF with one field: Make = "Test"
    let tiff = hx("49492a000800000001000f010200050000001a000000000000005465737400");
    let text = text_of(&run_bytes("exif_extract", "data", tiff, json!({})), "text");
    assert!(text.contains("Make"), "got: {text}");
    assert!(text.contains("Test"), "got: {text}");
}

#[test]
fn lsb_extract_flag() {
    // 24×1 PNG with "FLAG{lsb}" hidden in the RGB LSBs (row-major, MSB-first)
    let png = hx("89504e470d0a1a0a0000000d49484452000000180000000108020000004bccf9fc0000002849444154789c1d8bb10d00000c82e0ffa369aa936210212500a9dfdfecb9eb3b97c67a6be3a51907401a1004ff7e7b920000000049454e44ae426082");
    let text = text_of(&run_bytes("lsb_extract", "data", png, json!({ "channels": "RGB", "bit": 0 })), "text");
    assert_eq!(text, "FLAG{lsb}");
}

#[test]
fn pgp_armor_roundtrip() {
    let armored = text_of(&run("pgp_enarmor", "data", "hello pgp", json!({ "blockType": "MESSAGE" })), "text");
    assert!(armored.contains("-----BEGIN PGP MESSAGE-----"));
    let out = run("pgp_dearmor", "text", &armored, json!({}));
    assert_eq!(text_of(&out, "hex"), "68656c6c6f20706770"); // "hello pgp"
    assert_eq!(text_of(&out, "type"), "PGP MESSAGE");
    assert!(matches!(out.get("crcOk"), Some(PortValue::Bool(true))));
}

#[test]
fn timestamp_roundtrip() {
    assert_eq!(t("from_timestamp", "1516239022", json!({})), "2018-01-18 01:30:22 UTC");
    assert_eq!(t("to_timestamp", "2018-01-18 01:30:22", json!({})), "1516239022");
}

#[test]
fn jwt_decode_payload() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let payload = t("jwt_decode", jwt, json!({}));
    assert!(payload.contains("John Doe") && payload.contains("1234567890"), "got {payload}");
}

#[test]
fn hash_crack_dictionary() {
    let reg = default_registry();
    let bool_of = |m: &HashMap<String, PortValue>, k: &str| match m.get(k) {
        Some(PortValue::Bool(b)) => *b,
        o => panic!("expected Bool at '{k}', got {o:?}"),
    };
    let crack = |hash: &str, words: Vec<&str>, params: Value| {
        let mut inputs = HashMap::new();
        inputs.insert("hash".to_string(), PortValue::Text(hash.to_string()));
        inputs.insert(
            "wordlist".to_string(),
            PortValue::StringList(words.into_iter().map(String::from).collect()),
        );
        GraphExecutor::run_node(&reg, "hash_crack", &inputs, &params, &NullSink, &CancellationToken::new()).unwrap()
    };

    // MD5("password") — classic.
    let hit = crack(
        "5f4dcc3b5aa765d61d8327deb882cf99",
        vec!["admin", "123456", "password", "root"],
        json!({ "algorithm": "MD5" }),
    );
    assert_eq!(text_of(&hit, "text"), "password");
    assert!(bool_of(&hit, "found"));

    // Miss.
    let miss = crack("5f4dcc3b5aa765d61d8327deb882cf99", vec!["nope"], json!({ "algorithm": "MD5" }));
    assert_eq!(text_of(&miss, "text"), "");
    assert!(!bool_of(&miss, "found"));

    // Salted: target from the `hash` node = MD5("s4lt" + "secret"), then crack with prefix salt.
    let target = text_of(&run("hash", "data", "s4ltsecret", json!({ "algorithm": "MD5" })), "text");
    let salted = crack(&target, vec!["x", "secret"], json!({ "algorithm": "MD5", "salt": "s4lt", "saltMode": "前缀" }));
    assert_eq!(text_of(&salted, "text"), "secret");
}

#[test]
fn string_match_ops() {
    let b = |op: &str, text: &str, value: &str, ic: bool| {
        match run("string_match", "text", text, json!({ "operation": op, "value": value, "ignoreCase": ic })).get("result") {
            Some(PortValue::Bool(x)) => *x,
            o => panic!("expected Bool result, got {o:?}"),
        }
    };
    assert!(b("包含", "hello world", "world", false));
    assert!(!b("包含", "hello", "world", false));
    assert!(b("不包含", "abc", "z", false));
    assert!(b("等于", "FLAG", "flag", true));
    assert!(!b("等于", "FLAG", "flag", false));
    assert!(b("开头是", "flag{x}", "flag{", false));
    assert!(b("结尾是", "flag{x}", "}", false));
    assert!(b("正则匹配", "id=1234", r"\d+", false));
    assert!(b("正则不匹配", "abc", r"\d+", false));
}
