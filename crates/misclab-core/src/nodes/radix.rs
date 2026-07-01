//! Integer base (radix) conversion, base 2–36. Arbitrary precision via base-256
//! bignum digit arithmetic — like CyberChef's To Base / From Base combined.
use super::prelude::*;

const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

fn digit_val(c: char) -> Option<u32> {
    let c = c.to_ascii_lowercase();
    DIGITS.iter().position(|&d| d as char == c).map(|p| p as u32)
}

/// Parse `s` (base `from`) into a little-endian base-256 bignum.
fn parse_bignum(s: &str, from: u32) -> Result<Vec<u8>, CoreError> {
    let mut num: Vec<u8> = vec![0];
    for c in s.chars() {
        if c.is_whitespace() {
            continue;
        }
        let d = digit_val(c).ok_or_else(|| CoreError::Parse(format!("'{c}' 不是合法数字")))?;
        if d >= from {
            return Err(CoreError::Parse(format!("数字 '{c}' 超出 base{from} 范围")));
        }
        let mut carry = d;
        for b in num.iter_mut() {
            let v = *b as u32 * from + carry;
            *b = (v & 0xff) as u8;
            carry = v >> 8;
        }
        while carry > 0 {
            num.push((carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    Ok(num)
}

/// Render a little-endian base-256 bignum in base `to`.
fn format_bignum(mut num: Vec<u8>, to: u32) -> String {
    let mut out = Vec::new();
    while num.iter().any(|&b| b != 0) {
        let mut rem = 0u32;
        for b in num.iter_mut().rev() {
            let cur = (rem << 8) | *b as u32;
            *b = (cur / to) as u8;
            rem = cur % to;
        }
        out.push(DIGITS[rem as usize]);
    }
    if out.is_empty() {
        out.push(b'0');
    }
    out.reverse();
    String::from_utf8(out).expect("digits are ASCII")
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let input = in_text(inputs, "text")?.trim();
        let from = params.get("from").and_then(|v| v.as_f64()).unwrap_or(10.0) as u32;
        let to = params.get("to").and_then(|v| v.as_f64()).unwrap_or(16.0) as u32;
        if !(2..=36).contains(&from) || !(2..=36).contains(&to) {
            return Err(CoreError::Parse("进制需在 2–36 之间".into()));
        }
        if input.is_empty() {
            return Ok(out_text(String::new()));
        }
        Ok(out_text(format_bignum(parse_bignum(input, from)?, to)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "radix_convert",
            RADIX,
            "进制转换",
            SLATE,
            vec![req("text", "数字", PortType::Text)],
            vec![req("text", "结果", PortType::Text)],
            vec![
                ParamSpec::number("from", "源进制", 2.0, 36.0, 1.0, 10.0),
                ParamSpec::number("to", "目标进制", 2.0, 36.0, 1.0, 16.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
