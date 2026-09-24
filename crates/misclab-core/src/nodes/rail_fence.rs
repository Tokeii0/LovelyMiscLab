//! Rail Fence (zigzag) transposition cipher — encode / decode.
use super::prelude::*;

fn pattern(len: usize, rails: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(len);
    let (mut rail, mut dir) = (0i64, 1i64);
    for _ in 0..len {
        out.push(rail as usize);
        if rail == 0 {
            dir = 1;
        } else if rail == rails as i64 - 1 {
            dir = -1;
        }
        rail += dir;
    }
    out
}

fn encode(s: &str, rails: usize) -> String {
    if rails < 2 {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let pat = pattern(chars.len(), rails);
    let mut fence = vec![String::new(); rails];
    for (i, &c) in chars.iter().enumerate() {
        fence[pat[i]].push(c);
    }
    fence.concat()
}

fn decode(s: &str, rails: usize) -> String {
    if rails < 2 {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let pat = pattern(n, rails);
    // Distribute ciphertext across rails (rail r gets all positions where pat==r).
    let mut rail_chars: Vec<Vec<char>> = vec![Vec::new(); rails];
    let mut ci = 0;
    for (r, bucket) in rail_chars.iter_mut().enumerate() {
        for (p, &pr) in pat.iter().enumerate() {
            if pr == r {
                bucket.push(chars[ci]);
                ci += 1;
                let _ = p;
            }
        }
    }
    let mut pos = vec![0usize; rails];
    let mut out = String::with_capacity(n);
    for &r in &pat {
        out.push(rail_chars[r][pos[r]]);
        pos[r] += 1;
    }
    out
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let rails = pnum(params, "rails", 3.0).max(2.0) as usize;
        Ok(out_text(encode(in_text(inputs, "text")?, rails)))
    }
}
struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let rails = pnum(params, "rails", 3.0).max(2.0) as usize;
        Ok(out_text(decode(in_text(inputs, "text")?, rails)))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let rails = || ParamSpec::number("rails", "栏数", 2.0, 100.0, 1.0, 3.0);
    reg.register(
        desc(
            "rail_fence_encode",
            CRYPTO,
            "栅栏密码加密",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![rails()],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "rail_fence_decode",
            CRYPTO,
            "栅栏密码解密",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![rails()],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
