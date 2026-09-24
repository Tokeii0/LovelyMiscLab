//! HTML entities — encode / decode (named + numeric).
use super::prelude::*;

const NAMED: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
    ("copy", '©'),
    ("reg", '®'),
    ("trade", '™'),
    ("hellip", '…'),
    ("mdash", '—'),
    ("ndash", '–'),
    ("lsquo", '‘'),
    ("rsquo", '’'),
    ("ldquo", '“'),
    ("rdquo", '”'),
    ("times", '×'),
    ("divide", '÷'),
    ("deg", '°'),
    ("plusmn", '±'),
    ("cent", '¢'),
    ("pound", '£'),
    ("euro", '€'),
    ("yen", '¥'),
    ("sect", '§'),
    ("para", '¶'),
    ("middot", '·'),
    ("laquo", '«'),
    ("raquo", '»'),
];

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let all = pstr(params, "mode", "仅特殊字符") == "全部非ASCII";
        let mut out = String::new();
        for c in in_text(inputs, "text")?.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#39;"),
                c if all && !c.is_ascii() => out.push_str(&format!("&#{};", c as u32)),
                c => out.push(c),
            }
        }
        Ok(out_text(out))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let s = in_text(inputs, "text")?;
        let bytes = s.as_bytes();
        let mut out = String::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'&' {
                if let Some(semi) = s[i..].find(';') {
                    let ent = &s[i + 1..i + semi];
                    let decoded = if let Some(hex) =
                        ent.strip_prefix("#x").or_else(|| ent.strip_prefix("#X"))
                    {
                        u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                    } else if let Some(dec) = ent.strip_prefix('#') {
                        dec.parse::<u32>().ok().and_then(char::from_u32)
                    } else {
                        NAMED.iter().find(|(n, _)| *n == ent).map(|(_, c)| *c)
                    };
                    if let Some(c) = decoded {
                        out.push(c);
                        i += semi + 1;
                        continue;
                    }
                }
            }
            out.push(s[i..].chars().next().unwrap());
            i += s[i..].chars().next().unwrap().len_utf8();
        }
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "html_entity_encode",
            ENC,
            "HTML 实体编码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select(
                "mode",
                "范围",
                &["仅特殊字符", "全部非ASCII"],
                "仅特殊字符",
            )],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "html_entity_decode",
            ENC,
            "HTML 实体解码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
