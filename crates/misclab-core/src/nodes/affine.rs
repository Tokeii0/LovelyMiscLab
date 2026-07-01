//! Affine cipher over the Latin alphabet: E(x) = (a·x + b) mod 26.
use super::prelude::*;

fn mod_inv(a: i64, m: i64) -> Option<i64> {
    (1..m).find(|&x| (a * x).rem_euclid(m) == 1)
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let a = params.get("a").and_then(|v| v.as_f64()).unwrap_or(5.0) as i64;
        let b = params.get("b").and_then(|v| v.as_f64()).unwrap_or(8.0) as i64;
        let decrypt = pstr(params, "operation", "加密") == "解密";

        let a_inv = if decrypt {
            Some(mod_inv(a.rem_euclid(26), 26).ok_or_else(|| {
                CoreError::Parse(format!("a={a} 与 26 不互质，无法解密"))
            })?)
        } else {
            None
        };

        let s: String = in_text(inputs, "text")?
            .chars()
            .map(|c| {
                let base = if c.is_ascii_lowercase() {
                    b'a'
                } else if c.is_ascii_uppercase() {
                    b'A'
                } else {
                    return c;
                };
                let x = (c as u8 - base) as i64;
                let y = match a_inv {
                    Some(ai) => (ai * (x - b)).rem_euclid(26),
                    None => (a * x + b).rem_euclid(26),
                };
                (base + y as u8) as char
            })
            .collect();
        Ok(out_text(s))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "affine",
            CRYPTO,
            "仿射密码",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::select("operation", "操作", &["加密", "解密"], "加密"),
                ParamSpec::number("a", "a (与26互质)", 1.0, 25.0, 1.0, 5.0),
                ParamSpec::number("b", "b", 0.0, 25.0, 1.0, 8.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
