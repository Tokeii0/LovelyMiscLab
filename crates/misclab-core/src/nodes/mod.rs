//! Built-in nodes. **Convention: one node (or a tightly-related encode/decode
//! pair) per file**, each exposing `pub fn register(reg: &mut NodeRegistry)`.
//! To add a node: create `nodes/<id>.rs` (see `prelude` for the recipe), then add
//! `mod <id>;` and `<id>::register(reg);` below — it appears in the palette
//! automatically.

mod prelude;

mod ai_judge;
mod ai_vision;
mod archive_extract;
mod base64;
mod compare;
mod concat;
mod file_import;
mod file_output;
mod hex;
mod length;
mod loop_decode;
mod magic_decode;
mod qr_decode;
mod qr_encode;
mod regex_extract;
mod replace;
mod reverse;
mod rot13;
mod split;
mod switch;
mod text_input;
mod text_output;
mod text_score;
mod url;
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
    base64::register(reg);
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
    // control / logic
    switch::register(reg);
    compare::register(reg);
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
