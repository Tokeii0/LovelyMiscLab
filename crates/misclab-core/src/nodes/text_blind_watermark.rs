//! Extract a text watermark hidden by **text_blind_watermark** (guofei9987).
//! Two variants, auto-detected:
//!   • v1  a `DEL` (U+007F) is inserted after a character for each `1` bit;
//!   • v2  a contiguous run of two zero-width characters encodes the bits.
//! Either way the payload bytes are XOR-masked with `random.randint(0,255)`
//! seeded by the password, so faithful recovery needs CPython's Mersenne
//! Twister — reimplemented here (verified byte-exact against real output).
use num_bigint::BigUint;
use sha2::{Digest, Sha512};

use super::prelude::*;

/// CPython-compatible MT19937 (`random` module).
struct Mt {
    mt: [u32; 624],
    mti: usize,
}

impl Mt {
    fn new() -> Self {
        Mt {
            mt: [0; 624],
            mti: 625,
        }
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..624 {
            let p = self.mt[i - 1] ^ (self.mt[i - 1] >> 30);
            self.mt[i] = 1812433253u32.wrapping_mul(p).wrapping_add(i as u32);
        }
        self.mti = 624;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19650218);
        let (mut i, mut j) = (1usize, 0usize);
        let mut k = 624.max(key.len());
        while k > 0 {
            let p = self.mt[i - 1] ^ (self.mt[i - 1] >> 30);
            self.mt[i] = (self.mt[i] ^ p.wrapping_mul(1664525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                self.mt[0] = self.mt[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = 623;
        while k > 0 {
            let p = self.mt[i - 1] ^ (self.mt[i - 1] >> 30);
            self.mt[i] = (self.mt[i] ^ p.wrapping_mul(1566083941)).wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                self.mt[0] = self.mt[623];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x8000_0000;
    }

    fn seed_biguint(&mut self, n: &BigUint) {
        let mut key = n.to_u32_digits(); // little-endian words
        if key.is_empty() {
            key.push(0);
        }
        self.init_by_array(&key);
    }

    fn genrand(&mut self) -> u32 {
        if self.mti >= 624 {
            const MAG: [u32; 2] = [0, 0x9908_b0df];
            let m = &mut self.mt;
            for kk in 0..227 {
                let y = (m[kk] & 0x8000_0000) | (m[kk + 1] & 0x7fff_ffff);
                m[kk] = m[kk + 397] ^ (y >> 1) ^ MAG[(y & 1) as usize];
            }
            for kk in 227..623 {
                let y = (m[kk] & 0x8000_0000) | (m[kk + 1] & 0x7fff_ffff);
                m[kk] = m[kk - 227] ^ (y >> 1) ^ MAG[(y & 1) as usize];
            }
            let y = (m[623] & 0x8000_0000) | (m[0] & 0x7fff_ffff);
            m[623] = m[396] ^ (y >> 1) ^ MAG[(y & 1) as usize];
            self.mti = 0;
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    fn getrandbits(&mut self, k: u32) -> u32 {
        self.genrand() >> (32 - k)
    }

    /// `random.randint(0, 255)` → `_randbelow(256)`.
    fn randint_byte(&mut self) -> u8 {
        let k = 32 - 256u32.leading_zeros(); // 256.bit_length() == 9
        let mut r = self.getrandbits(k);
        while r >= 256 {
            r = self.getrandbits(k);
        }
        r as u8
    }
}

fn seed_int(password: &str) -> Option<Mt> {
    let n: BigUint = password.trim().parse().ok()?;
    let mut mt = Mt::new();
    mt.seed_biguint(&n);
    Some(mt)
}

fn seed_str(password: &str) -> Mt {
    // int.from_bytes(pw.encode() + sha512(pw.encode()).digest(), 'big')
    let enc = password.as_bytes();
    let mut buf = enc.to_vec();
    buf.extend_from_slice(&Sha512::digest(enc));
    let n = BigUint::from_bytes_be(&buf);
    let mut mt = Mt::new();
    mt.seed_biguint(&n);
    mt
}

const ZW: [char; 5] = ['\u{1d}', '\u{200b}', '\u{200c}', '\u{200d}', '\u{feff}'];
const SPECIAL: [char; 6] = ['\u{1d}', '\u{7f}', '\u{200b}', '\u{200c}', '\u{200d}', '\u{feff}'];

/// v1: `DEL` after a char = `1` bit. Returns the raw watermark bit string.
fn bits_v1(chars: &[char]) -> Option<String> {
    let mut bits = String::new();
    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] != '\u{7f}' {
            idx += 1;
            bits.push('0');
        } else {
            idx += 2;
            bits.push('1');
        }
    }
    let first = bits.find('1')?;
    let rev = bits.chars().rev().position(|c| c == '1')?;
    let last = bits.len() - rev;
    let (start, end) = (first + 1, last.checked_sub(1)?);
    if start >= end {
        return None;
    }
    Some(bits[start..end].to_string())
}

/// v2: a run of two zero-width chars; smaller codepoint = `0`.
fn bits_v2(chars: &[char]) -> Option<String> {
    let mut left = None;
    let mut right = chars.len();
    for (i, c) in chars.iter().enumerate() {
        if SPECIAL.contains(c) {
            if left.is_none() {
                left = Some(i);
            }
        } else if left.is_some() {
            right = i;
            break;
        }
    }
    let left = left?;
    let run = &chars[left..right];
    let mut distinct: Vec<char> = run.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    let one = *distinct.last()?; // larger codepoint = '1'
    Some(run.iter().map(|&c| if c == one { '1' } else { '0' }).collect())
}

fn deobfuscate(bits: &str, mt: &mut Mt) -> Option<String> {
    let n = bits.len() / 8;
    if n == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let byte = u8::from_str_radix(&bits[8 * i..8 * i + 8], 2).ok()?;
        out.push(byte ^ mt.randint_byte());
    }
    String::from_utf8(out).ok()
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let chars: Vec<char> = text.chars().collect();
        let password = pstr(p, "password", "");

        // Detect the variant.
        let is_zw = chars.iter().any(|c| ZW.contains(c));
        let bits = match pstr(p, "variant", "自动") {
            "DEL(chr127)" => bits_v1(&chars),
            "零宽字符" => bits_v2(&chars),
            _ if is_zw => bits_v2(&chars),
            _ => bits_v1(&chars),
        }
        .ok_or_else(|| CoreError::Other("未在文本中找到盲水印标记。".into()))?;

        // Seed order: honour pwType; 自动 tries int then str when numeric.
        let numeric = password.trim().parse::<BigUint>().is_ok();
        let order: Vec<bool> = match pstr(p, "pwType", "自动") {
            "整数" => vec![true],
            "字符串" => vec![false],
            _ if numeric => vec![true, false],
            _ => vec![false],
        };

        for use_int in order {
            let mut mt = if use_int {
                match seed_int(password) {
                    Some(m) => m,
                    None => continue,
                }
            } else {
                seed_str(password)
            };
            if let Some(wm) = deobfuscate(&bits, &mut mt) {
                let mut m = PortMap::new();
                m.insert("text".into(), PortValue::Text(wm));
                return Ok(m);
            }
        }
        Err(CoreError::Other(
            "解出的不是有效文本——密码或密码类型（整数/字符串）可能不对。".into(),
        ))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "text_blind_watermark",
            STEG,
            "文本盲水印提取",
            PURPLE,
            vec![req("text", "含水印文本", PortType::Text)],
            vec![req("text", "水印", PortType::Text)],
            vec![
                ParamSpec::text("password", "密码", "", false),
                ParamSpec::select("pwType", "密码类型", &["自动", "整数", "字符串"], "自动"),
                ParamSpec::select("variant", "变体", &["自动", "DEL(chr127)", "零宽字符"], "自动"),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_matches_cpython() {
        let mut m = seed_int("114514").unwrap();
        let seq: Vec<u8> = (0..8).map(|_| m.randint_byte()).collect();
        assert_eq!(seq, [119, 53, 105, 81, 147, 139, 69, 228]);
        let mut m2 = seed_str("secret");
        let seq2: Vec<u8> = (0..8).map(|_| m2.randint_byte()).collect();
        assert_eq!(seq2, [3, 66, 32, 224, 181, 132, 109, 12]);
    }

    fn extract(text: &str, pw: &str) -> Option<String> {
        let chars: Vec<char> = text.chars().collect();
        let is_zw = chars.iter().any(|c| ZW.contains(c));
        let bits = if is_zw { bits_v2(&chars) } else { bits_v1(&chars) }?;
        let mut mt = seed_int(pw)?;
        deobfuscate(&bits, &mut mt)
    }

    #[test]
    fn extracts_v1_del() {
        // real text_blind_watermark v1 output, watermark "OK", password 114514
        let t = "a\u{7f}bcd\u{7f}e\u{7f}f\u{7f}ghijk\u{7f}l\u{7f}m\u{7f}n\u{7f}o\u{7f}p\u{7f}qr\u{7f}stuvwxyz0123456789ABCDEFGHIJ";
        assert_eq!(extract(t, "114514").as_deref(), Some("OK"));
    }

    #[test]
    fn extracts_v2_zero_width() {
        // real TextBlindWatermark2 output, watermark "OK", password 114514
        let t = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH\u{200d}\u{200d}\u{feff}\u{feff}\u{feff}\u{200d}\u{200d}\u{200d}\u{200d}\u{feff}\u{feff}\u{feff}\u{feff}\u{feff}\u{feff}\u{200d}IJ";
        assert_eq!(extract(t, "114514").as_deref(), Some("OK"));
    }
}
