//! Base64 隐写提取：带 `=` 填充的 base64 串，最后一个数据字符有「空闲低位」被普通解码器
//! 忽略（1 个 `=` → 2 bit，2 个 `=` → 4 bit）。逐行取这些位、跨行拼接 → 隐藏字节。
use super::prelude::*;

const STD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        let alpha = if pstr(p, "alphabet", "标准") == "URL" { URL } else { STD };

        let mut bits: Vec<u8> = Vec::new();
        for line in text.lines() {
            let tok = line.trim();
            if tok.is_empty() {
                continue;
            }
            let pad = tok.chars().rev().take_while(|&c| c == '=').count();
            let spare = match pad {
                1 => 2u32,
                2 => 4,
                _ => 0,
            };
            if spare == 0 {
                continue;
            }
            let data = &tok[..tok.len() - pad];
            if let Some(last) = data.chars().last() {
                if let Some(val) = alpha.find(last) {
                    let v = val as u8; // 0..63
                    for k in (0..spare).rev() {
                        bits.push((v >> k) & 1);
                    }
                }
            }
        }

        let bytes: Vec<u8> = bits
            .chunks_exact(8)
            .map(|c| c.iter().fold(0u8, |acc, &b| (acc << 1) | b))
            .collect();

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(String::from_utf8_lossy(&bytes).into_owned()));
        m.insert("hex".into(), PortValue::Text(hex::encode(&bytes)));
        m.insert("bits".into(), PortValue::Number(bits.len() as f64));
        m.insert("bytes".into(), PortValue::Bytes(Arc::from(bytes.into_boxed_slice())));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "base64_stego",
            STEG,
            "Base64隐写提取",
            PURPLE,
            vec![req("text", "base64(多行)", PortType::Text)],
            vec![
                req("text", "隐藏文本", PortType::Text),
                opt("hex", "Hex", PortType::Text),
                opt("bits", "位数", PortType::Number),
                opt("bytes", "字节", PortType::Bytes),
            ],
            vec![ParamSpec::select("alphabet", "码表", &["标准", "URL"], "标准")],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    #[test]
    fn extracts_hidden_bits() {
        // 每行末尾字符低位藏 1 bit（这里用 2-pad 行，各藏 4 bit）。
        // 手工构造：字母表 index 的低 4 位 = 目标半字节。
        // "AA==" → 末字符 'A'(0) 低4位=0000；"AP==" → 'P'(15) 低4位=1111 → 0x0f。
        // 两行拼 8 bit = 0000 1111 = 0x0f。
        let input = "AA==\nAP==";
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(input.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "base64_stego",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(matches!(out.get("hex"), Some(PortValue::Text(s)) if s == "0f"));
    }
}
