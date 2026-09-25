//! Small helpers shared by every LLM-backed command (one-shot generate, agent,
//! port suggestions, 故事模式), so they parse model output the same way.

/// The outermost `{…}` in a model reply (models like to wrap JSON in prose or
/// code fences).
pub(crate) fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}

/// The outermost `[…]` in a model reply.
pub(crate) fn extract_json_array(s: &str) -> Option<&str> {
    let start = s.find('[')?;
    let end = s.rfind(']')?;
    (end > start).then(|| &s[start..=end])
}

/// At most `n` characters (not bytes), with an ellipsis when cut.
pub(crate) fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulls_json_out_of_prose() {
        assert_eq!(
            extract_json("好的：```json\n{\"a\":1}\n```"),
            Some("{\"a\":1}")
        );
        assert_eq!(extract_json_array("x [1,2] y"), Some("[1,2]"));
        assert_eq!(extract_json("no json"), None);
        assert_eq!(truncate("密码是123", 3), "密码是…");
    }
}
