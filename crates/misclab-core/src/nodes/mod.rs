//! Built-in nodes. **Convention: one node (or a tightly-related encode/decode
//! pair) per file**, each exposing `pub fn register(reg: &mut NodeRegistry)`.
//! To add a node: create `nodes/<id>.rs` (see `prelude` for the recipe), then add
//! `mod <id>;` and `<id>::register(reg);` below — it appears in the palette
//! automatically.

mod prelude;

mod basex;
mod xform;

mod adfgvx;
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
mod bcrypt;
mod binary;
mod bitwise;
mod charset;
mod compare;
mod charcode;
mod concat;
mod decimal;
mod enigma;
mod exif_meta;
mod extract;
mod file_import;
mod file_output;
mod filetype;
mod filter_list;
mod gate;
mod hash;
mod hex;
mod image_advanced;
mod image_blend;
mod image_channels;
mod image_colorspace;
mod image_diff;
mod image_filters;
mod image_freq;
mod image_geometry;
mod image_gif;
mod image_input;
mod image_meta;
mod image_util;
mod iterate;
mod join_list;
mod json_format;
mod jwt;
mod length;
mod logic;
mod loop_decode;
mod lsb_stego;
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
mod rsa;
mod split;
mod stegcloak;
mod switch;
mod switch_case;
mod text_input;
mod text_output;
mod text_score;
mod timestamp;
mod url;
mod vigenere;
mod xor;
mod xor_bruteforce;
mod zero_width;

mod a1z26;
mod bacon;
mod bifid;
mod bits_image;
mod blind_watermark;
mod pixel_image;
mod png_fix;
mod blowfish;
mod braille;
mod caesar;
mod chacha;
mod change_case;
mod char_freq;
mod defang;
mod deflate;
mod des;
mod entropy;
mod hexdump;
mod html_entity;
mod morse;
mod octal;
mod pad_lines;
mod pgp_armor;
mod pgp_decrypt;
mod playfair;
mod quoted_printable;
mod rail_fence;
mod regex_replace;
mod remove_whitespace;
mod rotate_bytes;
mod salsa;
mod sort_lines;
mod substitution;
mod substring;
mod unicode_escape;
mod unique_lines;
mod whitespace_stego;

use crate::node::registry::NodeRegistry;

/// Register every built-in node.
pub fn register_builtins(reg: &mut NodeRegistry) {
    // input / output
    text_input::register(reg);
    text_output::register(reg);
    file_import::register(reg);
    file_output::register(reg);
    image_input::register(reg);
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
    lsb_stego::register(reg);
    stegcloak::register(reg);
    whitespace_stego::register(reg);
    // hashes / MACs
    hash::register(reg);
    bcrypt::register(reg);
    // radix / number bases
    radix::register(reg);
    binary::register(reg);
    decimal::register(reg);
    charcode::register(reg);
    // character sets
    charset::register(reg);
    quoted_printable::register(reg);
    // ciphers
    aes::register(reg);
    rc4::register(reg);
    vigenere::register(reg);
    affine::register(reg);
    atbash::register(reg);
    rot47::register(reg);
    des::register(reg);
    blowfish::register(reg);
    chacha::register(reg);
    salsa::register(reg);
    rsa::register(reg);
    bifid::register(reg);
    playfair::register(reg);
    enigma::register(reg);
    adfgvx::register(reg);
    pgp_armor::register(reg);
    pgp_decrypt::register(reg);
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
    // CyberChef parity — text / classical cipher / format / util
    change_case::register(reg);
    remove_whitespace::register(reg);
    sort_lines::register(reg);
    unique_lines::register(reg);
    substring::register(reg);
    regex_replace::register(reg);
    pad_lines::register(reg);
    caesar::register(reg);
    rail_fence::register(reg);
    morse::register(reg);
    bacon::register(reg);
    a1z26::register(reg);
    html_entity::register(reg);
    unicode_escape::register(reg);
    hexdump::register(reg);
    octal::register(reg);
    entropy::register(reg);
    char_freq::register(reg);
    defang::register(reg);
    deflate::register(reg);
    jwt::register(reg);
    bitwise::register(reg);
    json_format::register(reg);
    substitution::register(reg);
    braille::register(reg);
    timestamp::register(reg);
    filetype::register(reg);
    extract::register(reg);
    rotate_bytes::register(reg);
    exif_meta::register(reg);
    // image processing
    image_channels::register(reg);
    image_blend::register(reg);
    image_filters::register(reg);
    image_geometry::register(reg);
    image_meta::register(reg);
    image_colorspace::register(reg);
    image_diff::register(reg);
    image_freq::register(reg);
    image_gif::register(reg);
    image_advanced::register(reg);
    png_fix::register(reg);
    blind_watermark::register(reg);
    bits_image::register(reg);
    pixel_image::register(reg);
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
