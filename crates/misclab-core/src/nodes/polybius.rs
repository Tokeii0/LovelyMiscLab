//! Polybius 方阵 与 敲击码(Tap code)：共用 5×5 网格（I/J 合并）。字母 → (行,列) 坐标。
//! - Polybius：坐标输出成数字对 `11 12` 或字母坐标 `AA AB`。
//! - Tap code：坐标输出成敲击点 `. .` / `..... .....`，字母间用 ` / ` 分隔。
use super::prelude::*;

const GRID: &str = "ABCDEFGHIKLMNOPQRSTUVWXYZ"; // J → I

/// 文本 → (行,列) 坐标（1..5）。
fn to_coords(text: &str) -> Vec<(usize, usize)> {
    text.to_uppercase()
        .chars()
        .map(|c| if c == 'J' { 'I' } else { c })
        .filter_map(|c| GRID.find(c).map(|idx| (idx / 5 + 1, idx % 5 + 1)))
        .collect()
}

/// (行,列) 坐标 → 文本。
fn from_coords(coords: &[(usize, usize)]) -> String {
    coords
        .iter()
        .filter_map(|&(r, c)| {
            if (1..=5).contains(&r) && (1..=5).contains(&c) {
                GRID.chars().nth((r - 1) * 5 + (c - 1))
            } else {
                None
            }
        })
        .collect()
}

/// 一个坐标字符 → 值（1..5）：数字 1-5 或字母 A-E。
fn coord_val(ch: char) -> Option<usize> {
    match ch {
        '1'..='5' => Some(ch as usize - '0' as usize),
        'A'..='E' => Some(ch as usize - 'A' as usize + 1),
        'a'..='e' => Some(ch as usize - 'a' as usize + 1),
        _ => None,
    }
}

// ------------------------------------------------------------- Polybius
struct Polybius;
impl Node for Polybius {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        let letters = pstr(p, "style", "数字") == "字母";
        let out = if pstr(p, "operation", "编码") == "编码" {
            to_coords(text)
                .iter()
                .map(|&(r, c)| {
                    if letters {
                        format!("{}{}", (b'A' + r as u8 - 1) as char, (b'A' + c as u8 - 1) as char)
                    } else {
                        format!("{r}{c}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            let coords: Vec<(usize, usize)> = text
                .split_whitespace()
                .filter_map(|tok| {
                    let mut it = tok.chars();
                    Some((coord_val(it.next()?)?, coord_val(it.next()?)?))
                })
                .collect();
            from_coords(&coords)
        };
        Ok(out_text(out))
    }
}

// ------------------------------------------------------------- Tap code
struct TapCode;
impl Node for TapCode {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        let out = if pstr(p, "operation", "编码") == "编码" {
            to_coords(text)
                .iter()
                .map(|&(r, c)| format!("{} {}", ".".repeat(r), ".".repeat(c)))
                .collect::<Vec<_>>()
                .join(" / ")
        } else {
            let coords: Vec<(usize, usize)> = text
                .split(['/', '\n'])
                .filter_map(|seg| {
                    let runs: Vec<usize> =
                        seg.split_whitespace().map(|g| g.chars().filter(|&c| c == '.').count()).collect();
                    if runs.len() >= 2 && runs[0] >= 1 && runs[1] >= 1 {
                        Some((runs[0], runs[1]))
                    } else {
                        None
                    }
                })
                .collect();
            from_coords(&coords)
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "polybius",
            CRYPTO,
            "Polybius方阵",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::select("operation", "操作", &["编码", "解码"], "编码"),
                ParamSpec::select("style", "坐标形式", &["数字", "字母"], "数字"),
            ],
        ),
        Arc::new(|| Arc::new(Polybius)),
    );
    reg.register(
        desc(
            "tap_code",
            CRYPTO,
            "敲击码",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select("operation", "操作", &["编码", "解码"], "编码")],
        ),
        Arc::new(|| Arc::new(TapCode)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    fn run(id: &str, text: &str, op: &str) -> String {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(text.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            id,
            &i,
            &serde_json::json!({ "operation": op }),
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
    fn polybius_roundtrip() {
        let enc = run("polybius", "HELLO", "编码");
        assert_eq!(run("polybius", &enc, "解码"), "HELLO");
    }

    #[test]
    fn tap_code_roundtrip() {
        let enc = run("tap_code", "HELLO", "编码");
        assert_eq!(run("tap_code", &enc, "解码"), "HELLO");
    }
}
