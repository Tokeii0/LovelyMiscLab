//! Textbook RSA — private-key recovery from (p, q, e) and decryption.
use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use num_traits::{Num, One};

use super::prelude::*;

/// Parse a big unsigned integer in decimal, or `0x…` hex.
fn parse_uint(s: &str) -> Option<BigUint> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        BigUint::from_str_radix(h, 16).ok()
    } else {
        BigUint::from_str_radix(s, 10).ok()
    }
}

/// Modular inverse via the extended Euclidean algorithm.
fn mod_inverse(a: &BigUint, m: &BigUint) -> Option<BigUint> {
    let eg = BigInt::from(a.clone()).extended_gcd(&BigInt::from(m.clone()));
    if !eg.gcd.is_one() {
        return None;
    }
    let mm = BigInt::from(m.clone());
    let mut x = eg.x % &mm;
    if x.sign() == Sign::Minus {
        x += &mm;
    }
    x.to_biguint()
}

fn derive_d(p: &BigUint, q: &BigUint, e: &BigUint) -> Result<(BigUint, BigUint), CoreError> {
    let one = BigUint::one();
    let phi = (p - &one) * (q - &one);
    let d = mod_inverse(e, &phi)
        .ok_or_else(|| CoreError::Parse("e 与 φ(n) 不互质，无法求 d".into()))?;
    Ok((p * q, d))
}

struct Params;
impl Node for Params {
    fn run(
        &self,
        _in: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let p =
            parse_uint(pstr(params, "p", "")).ok_or_else(|| CoreError::Parse("p 无效".into()))?;
        let q =
            parse_uint(pstr(params, "q", "")).ok_or_else(|| CoreError::Parse("q 无效".into()))?;
        let e = parse_uint(pstr(params, "e", "65537"))
            .ok_or_else(|| CoreError::Parse("e 无效".into()))?;
        let one = BigUint::one();
        let n = &p * &q;
        let phi = (&p - &one) * (&q - &one);
        let d = mod_inverse(&e, &phi)
            .ok_or_else(|| CoreError::Parse("e 与 φ(n) 不互质，无法求 d".into()))?;
        let mut m = PortMap::new();
        m.insert(
            "text".to_string(),
            PortValue::Text(format!("n = {n}\nphi = {phi}\nd = {d}")),
        );
        m.insert("n".to_string(), PortValue::Text(n.to_string()));
        m.insert("phi".to_string(), PortValue::Text(phi.to_string()));
        m.insert("d".to_string(), PortValue::Text(d.to_string()));
        Ok(m)
    }
}

struct Decrypt;
impl Node for Decrypt {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let c = parse_uint(in_text(inputs, "text")?)
            .ok_or_else(|| CoreError::Parse("密文 c 无效（需十进制或 0x 十六进制整数）".into()))?;

        let (n, d) = {
            let ns = pstr(params, "n", "");
            let ds = pstr(params, "d", "");
            if !ns.trim().is_empty() && !ds.trim().is_empty() {
                (
                    parse_uint(ns).ok_or_else(|| CoreError::Parse("n 无效".into()))?,
                    parse_uint(ds).ok_or_else(|| CoreError::Parse("d 无效".into()))?,
                )
            } else {
                let p = parse_uint(pstr(params, "p", ""))
                    .ok_or_else(|| CoreError::Parse("需要 (n,d) 或 (p,q,e)".into()))?;
                let q = parse_uint(pstr(params, "q", ""))
                    .ok_or_else(|| CoreError::Parse("需要 q".into()))?;
                let e = parse_uint(pstr(params, "e", "65537"))
                    .ok_or_else(|| CoreError::Parse("需要 e".into()))?;
                derive_d(&p, &q, &e)?
            }
        };

        let m = c.modpow(&d, &n);
        let bytes = m.to_bytes_be();
        let mut out = PortMap::new();
        out.insert(
            "text".to_string(),
            PortValue::Text(String::from_utf8_lossy(&bytes).into_owned()),
        );
        out.insert("int".to_string(), PortValue::Text(m.to_string()));
        out.insert("hex".to_string(), PortValue::Text(hex::encode(&bytes)));
        out.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
        );
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "rsa_params",
            CRYPTO,
            "RSA 参数计算",
            ROSE,
            vec![],
            vec![
                req("text", "摘要", PortType::Text),
                opt("n", "n", PortType::Text),
                opt("phi", "φ(n)", PortType::Text),
                opt("d", "d", PortType::Text),
            ],
            vec![
                ParamSpec::text("p", "素数 p", "", false),
                ParamSpec::text("q", "素数 q", "", false),
                ParamSpec::text("e", "公钥指数 e", "65537", false),
            ],
        ),
        Arc::new(|| Arc::new(Params)),
    );
    reg.register(
        desc(
            "rsa_decrypt",
            CRYPTO,
            "RSA 解密",
            ROSE,
            vec![req("text", "密文 c", PortType::Text)],
            vec![
                req("text", "明文", PortType::Text),
                opt("int", "整数 m", PortType::Text),
                opt("hex", "hex", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::text("n", "模数 n", "", false),
                ParamSpec::text("d", "私钥 d", "", false),
                ParamSpec::text("p", "素数 p(可选)", "", false),
                ParamSpec::text("q", "素数 q(可选)", "", false),
                ParamSpec::text("e", "e(配合 p,q)", "65537", false),
            ],
        ),
        Arc::new(|| Arc::new(Decrypt)),
    );
}
