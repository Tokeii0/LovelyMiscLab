//! Bitwise operations on bytes — NOT / shifts / rotates / AND·OR·XOR with a key.
use super::basex::decoded;
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "data")?;
        let n = (pnum(params, "amount", 1.0) as u32) % 8;
        let out: Vec<u8> = match pstr(params, "operation", "XOR") {
            "NOT" => data.iter().map(|b| !b).collect(),
            "左移" => data.iter().map(|b| b << n).collect(),
            "右移" => data.iter().map(|b| b >> n).collect(),
            "循环左移" => data.iter().map(|b| b.rotate_left(n)).collect(),
            "循环右移" => data.iter().map(|b| b.rotate_right(n)).collect(),
            op => {
                let key = parse_bytes(pstr(params, "key", ""), "Hex")?;
                if key.is_empty() {
                    data.to_vec()
                } else {
                    data.iter()
                        .enumerate()
                        .map(|(i, &b)| {
                            let k = key[i % key.len()];
                            match op {
                                "AND" => b & k,
                                "OR" => b | k,
                                _ => b ^ k,
                            }
                        })
                        .collect()
                }
            }
        };
        Ok(decoded(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "bitwise",
            ENC,
            "位运算",
            BLUE,
            vec![req("data", "输入", PortType::Any)],
            vec![
                req("text", "文本", PortType::Text),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![
                ParamSpec::select(
                    "operation",
                    "运算",
                    &[
                        "XOR",
                        "AND",
                        "OR",
                        "NOT",
                        "左移",
                        "右移",
                        "循环左移",
                        "循环右移",
                    ],
                    "XOR",
                ),
                ParamSpec::text("key", "密钥(Hex, 用于 AND/OR/XOR)", "", false),
                ParamSpec::number("amount", "位数(用于移位)", 0.0, 7.0, 1.0, 1.0),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
