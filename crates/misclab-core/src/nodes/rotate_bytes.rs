//! Rotate bits left/right — per byte, or across the whole buffer with carry.
//! Mirrors CyberChef's "Rotate left / Rotate right".
use super::prelude::*;

fn rotate_carry(data: &[u8], amount: usize, left: bool) -> Vec<u8> {
    let bits = data.len() * 8;
    if bits == 0 {
        return Vec::new();
    }
    let shift = if left {
        amount % bits
    } else {
        (bits - amount % bits) % bits
    };
    let get = |i: usize| (data[i / 8] >> (7 - i % 8)) & 1;
    let mut out = vec![0u8; data.len()];
    for i in 0..bits {
        if get((i + shift) % bits) == 1 {
            out[i / 8] |= 1 << (7 - i % 8);
        }
    }
    out
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let left = pstr(p, "direction", "左(ROL)").starts_with('左');
        let amount = pnum(p, "amount", 1.0).max(0.0) as usize;
        let out: Vec<u8> = if pbool(p, "carry", false) {
            rotate_carry(&data, amount, left)
        } else {
            let n = (amount % 8) as u32;
            data.iter()
                .map(|&b| {
                    if left {
                        b.rotate_left(n)
                    } else {
                        b.rotate_right(n)
                    }
                })
                .collect()
        };
        let mut m = PortMap::new();
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(out.clone().into_boxed_slice())),
        );
        m.insert("hex".to_string(), PortValue::Text(hex::encode(&out)));
        m.insert(
            "text".to_string(),
            PortValue::Text(String::from_utf8_lossy(&out).into_owned()),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "rotate_bytes",
            UTIL,
            "位旋转 (ROL/ROR)",
            AMBER,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("bytes", "字节", PortType::Bytes),
                opt("hex", "hex", PortType::Text),
                opt("text", "文本", PortType::Text),
            ],
            vec![
                ParamSpec::select("direction", "方向", &["左(ROL)", "右(ROR)"], "左(ROL)"),
                ParamSpec::number("amount", "位数", 0.0, 64.0, 1.0, 1.0),
                ParamSpec::toggle("carry", "跨字节进位", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
