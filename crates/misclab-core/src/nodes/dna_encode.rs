//! DNA 编码：每 2 bit ↔ 一个碱基（默认 A=00 G=01 C=10 T=11，映射可配）。CTF 里一串
//! ATCG 常是二进制→ASCII。编码：文本→字节→碱基；解码：碱基→字节→文本。
use super::prelude::*;

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        let map: Vec<char> = pstr(p, "mapping", "AGCT").chars().collect();
        if map.len() != 4 {
            return Err(CoreError::Parse("映射必须是 4 个字符（对应 00/01/10/11）".into()));
        }

        let out = if pstr(p, "operation", "解码") == "编码" {
            let mut s = String::new();
            for &byte in text.as_bytes() {
                for shift in [6, 4, 2, 0] {
                    s.push(map[((byte >> shift) & 0b11) as usize]);
                }
            }
            s
        } else {
            // 碱基 → 2bit → 字节。
            let bits: Vec<u8> = text
                .chars()
                .filter_map(|c| map.iter().position(|&m| m.eq_ignore_ascii_case(&c) || m == c).map(|v| v as u8))
                .collect();
            let bytes: Vec<u8> = bits
                .chunks_exact(4)
                .map(|q| (q[0] << 6) | (q[1] << 4) | (q[2] << 2) | q[3])
                .collect();
            String::from_utf8_lossy(&bytes).into_owned()
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "dna_encode",
            ENC,
            "DNA(ATCG)",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::select("operation", "操作", &["解码", "编码"], "解码"),
                ParamSpec::text("mapping", "映射(00/01/10/11)", "AGCT", false),
            ],
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

    fn run(text: &str, op: &str) -> String {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(text.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "dna_encode",
            &i,
            &serde_json::json!({ "operation": op, "mapping": "AGCT" }),
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
    fn roundtrip() {
        let enc = run("flag", "编码");
        assert!(enc.chars().all(|c| "AGCT".contains(c)));
        assert_eq!(run(&enc, "解码"), "flag");
    }
}
