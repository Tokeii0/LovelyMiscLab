//! Built-in nodes. **Convention: one node (or a tightly-related encode/decode
//! pair) per file**, each exposing `pub fn register(reg: &mut NodeRegistry)`.
//! To add a node: create `nodes/<id>.rs` (see `prelude` for the recipe), then add
//! `mod <id>;` and `<id>::register(reg);` below — it appears in the palette
//! automatically.

mod prelude;

mod basex;
mod xform;

mod aes;
mod affine;
mod ai_judge;
mod ai_vision;
mod archive_extract;
mod atbash;
mod base32;
mod base45;
mod base58;
mod base62;
mod base64;
mod base85;
mod base92;
mod binary;
mod charset;
mod compare;
mod concat;
mod decimal;
mod file_import;
mod file_output;
mod filter_list;
mod gate;
mod hash;
mod hex;
mod iterate;
mod join_list;
mod length;
mod logic;
mod loop_decode;
mod magic_decode;
mod map;
mod qr_decode;
mod qr_encode;
mod radix;
mod range;
mod rc4;
mod regex_extract;
mod replace;
mod reverse;
mod rot13;
mod rot47;
mod split;
mod switch;
mod switch_case;
mod text_input;
mod text_output;
mod text_score;
mod url;
mod vigenere;
mod xor;
mod xor_bruteforce;
mod zero_width;

use crate::node::registry::NodeRegistry;

/// Register every built-in node.
pub fn register_builtins(reg: &mut NodeRegistry) {
    // input / output
    text_input::register(reg);
    text_output::register(reg);
    file_import::register(reg);
    file_output::register(reg);
    // encoding / crypto
    base32::register(reg);
    base45::register(reg);
    base58::register(reg);
    base62::register(reg);
    base64::register(reg);
    base85::register(reg);
    base92::register(reg);
    hex::register(reg);
    url::register(reg);
    rot13::register(reg);
    xor::register(reg);
    xor_bruteforce::register(reg);
    loop_decode::register(reg);
    magic_decode::register(reg);
    qr_encode::register(reg);
    qr_decode::register(reg);
    // text processing
    reverse::register(reg);
    regex_extract::register(reg);
    text_score::register(reg);
    concat::register(reg);
    split::register(reg);
    length::register(reg);
    replace::register(reg);
    // archives
    archive_extract::register(reg);
    // steganography
    zero_width::register(reg);
    // hashes / MACs
    hash::register(reg);
    // radix / number bases
    radix::register(reg);
    binary::register(reg);
    decimal::register(reg);
    // character sets
    charset::register(reg);
    // ciphers
    aes::register(reg);
    rc4::register(reg);
    vigenere::register(reg);
    affine::register(reg);
    atbash::register(reg);
    rot47::register(reg);
    // control / logic
    switch::register(reg);
    switch_case::register(reg);
    compare::register(reg);
    logic::register(reg);
    gate::register(reg);
    range::register(reg);
    map::register(reg);
    filter_list::register(reg);
    join_list::register(reg);
    iterate::register(reg);
    // ai
    ai_judge::register(reg);
    ai_vision::register(reg);
}

/// A registry pre-loaded with all built-in nodes.
pub fn default_registry() -> NodeRegistry {
    let mut reg = NodeRegistry::new();
    register_builtins(&mut reg);
    reg
}
