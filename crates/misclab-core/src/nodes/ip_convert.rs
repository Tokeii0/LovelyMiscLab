//! IP 地址转换：自动识别点分 `1.2.3.4` / 十进制整数 `16909060` / 十六进制 `0x01020304`，
//! 输出全部三种形式。CTF 里 IP 常被写成一个大整数或十六进制来藏。
use super::prelude::*;

fn parse_ip(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.contains('.') {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return None;
        }
        let mut v = 0u32;
        for p in parts {
            v = (v << 8) | p.trim().parse::<u8>().ok()? as u32;
        }
        return Some(v);
    }
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u32::from_str_radix(h, 16).ok();
    }
    s.parse::<u32>().ok()
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let v = parse_ip(in_text(i, "text")?)
            .ok_or_else(|| CoreError::Parse("无法识别为 IPv4（点分/十进制/0x十六进制）".into()))?;
        let dotted = format!("{}.{}.{}.{}", v >> 24, (v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff);
        let decimal = v.to_string();
        let hexs = format!("0x{v:08x}");

        let mut m = PortMap::new();
        m.insert("text".into(), PortValue::Text(format!("点分：{dotted}\n十进制：{decimal}\n十六进制：{hexs}")));
        m.insert("dotted".into(), PortValue::Text(dotted));
        m.insert("decimal".into(), PortValue::Text(decimal));
        m.insert("hex".into(), PortValue::Text(hexs));
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "ip_convert",
            UTIL,
            "IP地址转换",
            BLUE,
            vec![req("text", "IP", PortType::Text)],
            vec![
                req("text", "全部形式", PortType::Text),
                opt("dotted", "点分", PortType::Text),
                opt("decimal", "十进制", PortType::Text),
                opt("hex", "十六进制", PortType::Text),
            ],
            vec![],
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

    fn dotted(input: &str) -> String {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(input.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "ip_convert",
            &i,
            &serde_json::json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("dotted") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn all_forms_agree() {
        assert_eq!(dotted("1.2.3.4"), "1.2.3.4");
        assert_eq!(dotted("16909060"), "1.2.3.4");
        assert_eq!(dotted("0x01020304"), "1.2.3.4");
    }
}
