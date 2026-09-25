//! 哈希爆破（字典攻击）：给定目标哈希与一份字典，逐个候选词算哈希比对，命中即得明文。
//! 支持可选加盐（前缀/后缀），复用 `hash` 节点的算法实现。CTF 里「已知哈希求明文」常用。
use super::hash::hash_hex;
use super::prelude::*;

const ALGOS: &[&str] = &[
    "MD5",
    "SHA1",
    "SHA256",
    "SHA512",
    "SHA224",
    "SHA384",
    "SHA3-256",
    "SHA3-512",
    "Keccak-256",
    "RIPEMD-160",
    "SM3",
    "MD4",
    "BLAKE2b",
    "BLAKE2s",
    "Whirlpool",
    "CRC32",
];

struct Crack;
impl Node for Crack {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let target = in_text(i, "hash")?.trim().to_ascii_lowercase();
        if target.is_empty() {
            return Err(CoreError::Parse("目标哈希为空。".into()));
        }
        let words = in_list(i, "wordlist")?;
        let algo = pstr(p, "algorithm", "MD5");
        let salt = pstr(p, "salt", "");
        let salt_mode = pstr(p, "saltMode", "无");

        let mut hit: Option<String> = None;
        for (n, word) in words.iter().enumerate() {
            if n % 2000 == 0 {
                ctx.check_cancel()?;
            }
            let candidate = match salt_mode {
                "前缀" => format!("{salt}{word}"),
                "后缀" => format!("{word}{salt}"),
                _ => word.clone(),
            };
            if hash_hex(algo, candidate.as_bytes())? == target {
                hit = Some(word.clone());
                break;
            }
        }

        let mut m = PortMap::new();
        match &hit {
            Some(w) => {
                m.insert("text".into(), PortValue::Text(w.clone()));
                m.insert("found".into(), PortValue::Bool(true));
                m.insert(
                    "report".into(),
                    PortValue::Text(format!(
                        "命中！{algo}(明文)= {target}，明文 = \"{w}\"（试了 {} 个候选）。",
                        words.len()
                    )),
                );
            }
            None => {
                m.insert("text".into(), PortValue::Text(String::new()));
                m.insert("found".into(), PortValue::Bool(false));
                m.insert(
                    "report".into(),
                    PortValue::Text(format!(
                        "字典 {} 个候选均未命中（算法 {algo}）。换算法或扩充字典再试。",
                        words.len()
                    )),
                );
            }
        }
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "hash_crack",
            HASH,
            "哈希爆破",
            CYAN,
            vec![
                req("hash", "目标哈希(hex)", PortType::Text),
                req("wordlist", "字典", PortType::Any),
            ],
            vec![
                req("text", "明文", PortType::Text),
                opt("found", "命中", PortType::Bool),
                opt("report", "信息", PortType::Text),
            ],
            vec![
                ParamSpec::select("algorithm", "算法", ALGOS, "MD5"),
                ParamSpec::text("salt", "盐(可选)", "", false),
                ParamSpec::select("saltMode", "加盐位置", &["无", "前缀", "后缀"], "无"),
            ],
        )
        .heavy(),
        Arc::new(|| Arc::new(Crack)),
    );
}
