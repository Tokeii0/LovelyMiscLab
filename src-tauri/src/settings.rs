//! Persistence for app settings (AI model config + default output dir) in a JSON
//! file under the app data dir.

use std::path::Path;

use misclab_core::node::NodeEnv;

const FILE: &str = "settings.json";

/// Read settings. A file that exists but doesn't parse is moved aside to
/// `settings.json.bak` (so the next save can't destroy it) and defaults are used.
pub fn load(dir: &Path) -> NodeEnv {
    let path = dir.join(FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return NodeEnv::default();
    };
    match serde_json::from_str(&text) {
        Ok(env) => env,
        Err(e) => {
            let backup = dir.join(format!("{FILE}.bak"));
            eprintln!(
                "settings.json unreadable ({e}); moved to {}",
                backup.display()
            );
            std::fs::rename(&path, &backup).ok();
            NodeEnv::default()
        }
    }
}

pub fn save(dir: &Path, settings: &NodeEnv) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    write_atomic(&dir.join(FILE), json.as_bytes())
}

/// Write via a temp file + rename, so a crash mid-write never leaves a
/// truncated file behind.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_settings_are_backed_up_not_lost() {
        let dir = std::env::temp_dir().join(format!("misclab_settings_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE), "{ not json").unwrap();
        let env = load(&dir);
        assert!(env.ai.llm.model.is_empty());
        assert_eq!(
            std::fs::read_to_string(dir.join("settings.json.bak")).unwrap(),
            "{ not json"
        );
        save(&dir, &env).unwrap();
        assert!(load(&dir).ai.llm.model.is_empty());
        assert!(!dir.join("settings.json.tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
