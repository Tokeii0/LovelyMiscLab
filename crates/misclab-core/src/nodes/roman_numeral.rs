//! 罗马数字 ↔ 整数（1..3999）。
use super::prelude::*;

const VALS: [(u32, &str); 13] = [
    (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
    (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
];

fn to_roman(mut n: u32) -> String {
    let mut s = String::new();
    for &(v, sym) in &VALS {
        while n >= v {
            s.push_str(sym);
            n -= v;
        }
    }
    s
}

fn char_val(c: char) -> Option<u32> {
    match c.to_ascii_uppercase() {
        'I' => Some(1),
        'V' => Some(5),
        'X' => Some(10),
        'L' => Some(50),
        'C' => Some(100),
        'D' => Some(500),
        'M' => Some(1000),
        _ => None,
    }
}

fn from_roman(s: &str) -> Option<u32> {
    let vals: Vec<u32> = s.trim().chars().filter_map(char_val).collect();
    if vals.is_empty() {
        return None;
    }
    let mut total = 0i64;
    for i in 0..vals.len() {
        if i + 1 < vals.len() && vals[i] < vals[i + 1] {
            total -= vals[i] as i64;
        } else {
            total += vals[i] as i64;
        }
    }
    u32::try_from(total).ok()
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?.trim().to_string();
        let out = if pstr(p, "operation", "数字→罗马") == "罗马→数字" {
            from_roman(&text)
                .ok_or_else(|| CoreError::Parse("不是有效的罗马数字".into()))?
                .to_string()
        } else {
            let n: u32 = text.parse().map_err(|_| CoreError::Parse("请输入整数".into()))?;
            if n == 0 || n > 3999 {
                return Err(CoreError::Parse("罗马数字范围 1..3999".into()));
            }
            to_roman(n)
        };
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "roman_numeral",
            ENC,
            "罗马数字",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select("operation", "操作", &["数字→罗马", "罗马→数字"], "数字→罗马")],
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
            "roman_numeral",
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
    fn both_ways() {
        assert_eq!(run("2023", "数字→罗马"), "MMXXIII");
        assert_eq!(run("MMXXIII", "罗马→数字"), "2023");
        assert_eq!(run("MCMXCIV", "罗马→数字"), "1994");
    }
}
