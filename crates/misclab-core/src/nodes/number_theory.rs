//! 数论工具（大整数，复用 num-bigint）：`number_theory`(gcd/lcm/模逆/模幂/egcd)、
//! `factorize`(整数分解：试除 + Fermat + Pollard rho，攻弱/小 n)、`crt`(中国剩余定理)。
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::{One, Num, Signed, Zero};

use super::prelude::*;

/// 解析大整数：十进制或 `0x` 十六进制（可带负号）。
fn parse_int(s: &str) -> Option<BigInt> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        BigInt::from_str_radix(h, 16).ok()
    } else {
        BigInt::from_str_radix(s, 10).ok()
    }
}

fn read_int(i: &PortMap, name: &str) -> Option<BigInt> {
    match i.get(name) {
        Some(PortValue::Text(s)) => parse_int(s),
        Some(PortValue::Number(n)) => Some(BigInt::from(*n as i64)),
        _ => None,
    }
}

/// 扩展欧几里得：返回 (gcd, x, y)，满足 a·x + b·y = gcd。
fn egcd(a: &BigInt, b: &BigInt) -> (BigInt, BigInt, BigInt) {
    if b.is_zero() {
        (a.clone(), BigInt::one(), BigInt::zero())
    } else {
        let (g, x, y) = egcd(b, &(a % b));
        (g, y.clone(), x - (a / b) * y)
    }
}

/// 模逆：a^-1 mod m（归一化为 [0,m)）。
fn modinv(a: &BigInt, m: &BigInt) -> Option<BigInt> {
    let (g, x, _) = egcd(a, m);
    if !g.is_one() {
        return None;
    }
    Some(x.mod_floor(m))
}

// ------------------------------------------------------------- 数论运算
struct NumberTheory;
impl Node for NumberTheory {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let a = read_int(i, "a").ok_or_else(|| CoreError::Parse("需要输入 a（整数）".into()))?;
        let need = |name: &str| read_int(i, name).ok_or_else(|| CoreError::Parse(format!("需要输入 {name}")));

        let result = match pstr(p, "op", "gcd") {
            "gcd" => egcd(&a, &need("b")?).0.abs().to_string(),
            "lcm" => {
                let b = need("b")?;
                let g = egcd(&a, &b).0.abs();
                if g.is_zero() {
                    "0".into()
                } else {
                    (&a / &g * &b).abs().to_string()
                }
            }
            "modinv" => modinv(&a, &need("m")?)
                .ok_or_else(|| CoreError::Parse("逆元不存在（a 与 m 不互质）".into()))?
                .to_string(),
            "modpow" => {
                let (b, m) = (need("b")?, need("m")?);
                let (au, bu, mu) = (
                    a.to_biguint().ok_or_else(|| CoreError::Parse("a 需非负".into()))?,
                    b.to_biguint().ok_or_else(|| CoreError::Parse("b 需非负".into()))?,
                    m.to_biguint().ok_or_else(|| CoreError::Parse("m 需非负".into()))?,
                );
                au.modpow(&bu, &mu).to_string()
            }
            "egcd" => {
                let (g, x, y) = egcd(&a, &need("b")?);
                format!("gcd = {g}\nx = {x}\ny = {y}")
            }
            o => return Err(CoreError::Parse(format!("未知运算: {o}"))),
        };
        Ok(out_text(result))
    }
}

// ------------------------------------------------------------- 整数分解
fn fermat(n: &BigUint, iters: u64) -> Option<(BigUint, BigUint)> {
    if n.is_even() {
        return None;
    }
    let mut a = n.sqrt() + BigUint::one();
    for _ in 0..iters {
        let b2 = &a * &a - n;
        let b = b2.sqrt();
        if &b * &b == b2 {
            return Some((&a - &b, &a + &b));
        }
        a += BigUint::one();
    }
    None
}

fn pollard_rho(n: &BigUint, iters: u64) -> Option<BigUint> {
    if n.is_even() {
        return Some(BigUint::from(2u32));
    }
    let one = BigUint::one();
    let c = BigUint::one();
    let f = |x: &BigUint| (x * x + &c) % n;
    let (mut x, mut y, mut d) = (BigUint::from(2u32), BigUint::from(2u32), one.clone());
    let mut count = 0u64;
    while d.is_one() && count < iters {
        x = f(&x);
        y = f(&f(&y));
        let diff = if x >= y { &x - &y } else { &y - &x };
        d = diff.gcd(n);
        count += 1;
    }
    if d.is_one() || &d == n {
        None
    } else {
        Some(d)
    }
}

fn factorize(n0: &BigUint, ctx: &NodeCtx) -> Result<Vec<BigUint>, CoreError> {
    const TRIAL: u64 = 200_000;
    let mut factors = Vec::new();
    let mut todo = vec![n0.clone()];
    let mut guard = 0u32;
    while let Some(mut n) = todo.pop() {
        guard += 1;
        if guard > 10_000 {
            break;
        }
        ctx.check_cancel()?;
        if n <= BigUint::one() {
            continue;
        }
        // 试除到 TRIAL，找一个小因子就拆出继续。
        let mut d = 2u64;
        let mut split = false;
        while d <= TRIAL {
            let bd = BigUint::from(d);
            if &bd * &bd > n {
                break;
            }
            if (&n % &bd).is_zero() {
                factors.push(bd.clone());
                n /= &bd;
                split = true;
                break;
            }
            d += if d == 2 { 1 } else { 2 };
        }
        if split {
            todo.push(n);
            continue;
        }
        // 无 ≤TRIAL 的因子：n 是 1/素数/大素数之积。
        if n <= BigUint::from(TRIAL) * BigUint::from(TRIAL) {
            factors.push(n); // 素数
            continue;
        }
        if let Some((a, b)) = fermat(&n, 200_000) {
            todo.push(a);
            todo.push(b);
            continue;
        }
        if let Some(f) = pollard_rho(&n, 1_000_000) {
            let other = &n / &f;
            todo.push(f);
            todo.push(other);
            continue;
        }
        factors.push(n); // 未能进一步分解
    }
    factors.sort();
    Ok(factors)
}

struct Factorize;
impl Node for Factorize {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let n = parse_int(in_text(i, "text")?)
            .and_then(|v| v.to_biguint())
            .ok_or_else(|| CoreError::Parse("请输入非负整数 n（十进制或 0x）".into()))?;
        if n <= BigUint::one() {
            return Err(CoreError::Parse("n 需大于 1".into()));
        }
        let factors = factorize(&n, ctx)?;
        let text = factors.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(" × ");
        let list: Vec<String> = factors.iter().map(|f| f.to_string()).collect();

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(text));
        m.insert("factors".into(), PortValue::StringList(list));
        Ok(m)
    }
}

// ------------------------------------------------------------- CRT
fn parse_list(s: &str) -> Vec<BigInt> {
    s.split([',', ' ', '\n', '\r', '\t', ';'])
        .filter(|t| !t.trim().is_empty())
        .filter_map(parse_int)
        .collect()
}

fn crt(rs: &[BigInt], ms: &[BigInt]) -> Option<(BigInt, BigInt)> {
    let mut x = rs[0].clone();
    let mut m = ms[0].clone();
    for k in 1..rs.len() {
        let (r2, m2) = (&rs[k], &ms[k]);
        let g = m.gcd(m2);
        let diff = r2 - &x;
        if !(&diff % &g).is_zero() {
            return None; // 无解（模不互质且不一致）
        }
        let mg = &m / &g;
        let m2g = m2 / &g;
        let inv = modinv(&mg.mod_floor(&m2g), &m2g)?;
        let t = ((&diff / &g) * inv).mod_floor(&m2g);
        x = &x + &m * t;
        m = &m / &g * m2; // lcm
        x = x.mod_floor(&m);
    }
    Some((x, m))
}

struct Crt;
impl Node for Crt {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let rs = parse_list(in_text(i, "remainders")?);
        let ms = parse_list(in_text(i, "moduli")?);
        if rs.is_empty() || rs.len() != ms.len() {
            return Err(CoreError::Parse("余数与模数数量需相等且非空".into()));
        }
        let (x, m) = crt(&rs, &ms).ok_or_else(|| CoreError::Parse("无解（模数不互质且不一致）".into()))?;
        let mut out = PortMap::new();
        out.insert("text".into(), PortValue::Text(format!("x = {x}  (mod {m})")));
        out.insert("x".into(), PortValue::Text(x.to_string()));
        out.insert("modulus".into(), PortValue::Text(m.to_string()));
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "number_theory",
            CRYPTO,
            "数论运算",
            INDIGO,
            vec![
                req("a", "a", PortType::Text),
                opt("b", "b", PortType::Text),
                opt("m", "m(模)", PortType::Text),
            ],
            vec![req("text", "结果", PortType::Text)],
            vec![ParamSpec::select(
                "op",
                "运算",
                &["gcd", "lcm", "modinv", "modpow", "egcd"],
                "gcd",
            )],
        ),
        Arc::new(|| Arc::new(NumberTheory)),
    );
    reg.register(
        {
            let mut d = desc(
                "factorize",
                CRYPTO,
                "整数分解",
                INDIGO,
                vec![req("text", "n", PortType::Text)],
                vec![
                    req("text", "因子", PortType::Text),
                    opt("factors", "因子列表", PortType::StringList),
                ],
                vec![],
            );
            d.cost = Cost::Heavy;
            d
        },
        Arc::new(|| Arc::new(Factorize)),
    );
    reg.register(
        desc(
            "crt",
            CRYPTO,
            "中国剩余定理",
            INDIGO,
            vec![
                req("remainders", "余数列表", PortType::Text),
                req("moduli", "模数列表", PortType::Text),
            ],
            vec![
                req("text", "结果", PortType::Text),
                opt("x", "x", PortType::Text),
                opt("modulus", "合并模数", PortType::Text),
            ],
            vec![],
        ),
        Arc::new(|| Arc::new(Crt)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    fn nt(op: &str, a: &str, b: &str, m: &str) -> String {
        let mut i = PortMap::new();
        i.insert("a".into(), PortValue::Text(a.into()));
        if !b.is_empty() {
            i.insert("b".into(), PortValue::Text(b.into()));
        }
        if !m.is_empty() {
            i.insert("m".into(), PortValue::Text(m.into()));
        }
        let out = GraphExecutor::run_node(
            &default_registry(),
            "number_theory",
            &i,
            &serde_json::json!({ "op": op }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("text") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn number_theory_ops() {
        assert_eq!(nt("gcd", "12", "18", ""), "6");
        assert_eq!(nt("lcm", "4", "6", ""), "12");
        assert_eq!(nt("modinv", "3", "", "11"), "4");
        assert_eq!(nt("modpow", "2", "10", "1000"), "24");
    }

    #[test]
    fn factorize_small() {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text("8051".into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "factorize",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("factors"), Some(PortValue::StringList(v)) if *v == vec!["83".to_string(), "97".to_string()]));
    }

    #[test]
    fn crt_solves() {
        let mut i = PortMap::new();
        i.insert("remainders".into(), PortValue::Text("2 3 2".into()));
        i.insert("moduli".into(), PortValue::Text("3 5 7".into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "crt",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("x"), Some(PortValue::Text(s)) if s == "23"));
    }
}
