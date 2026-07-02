//! Persistence for user-defined modules (composite sub-graphs and script nodes):
//! one JSON file per module under `<app_data_dir>/<subdir>/`. Generic over the
//! module type so both kinds share the same store.

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

fn dir(base: &Path, subdir: &str) -> PathBuf {
    base.join(subdir)
}

/// Keep filenames safe — ids are frontend-generated, but be defensive.
fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

/// Load every `*.json` module in `<base>/<subdir>` (skips anything unparsable).
/// Order is filesystem-dependent; callers sort as needed.
pub fn load_all<T: DeserializeOwned>(base: &Path, subdir: &str) -> Vec<T> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir(base, subdir)) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(m) = serde_json::from_str::<T>(&s) {
                    out.push(m);
                }
            }
        }
    }
    out
}

pub fn save_one<T: Serialize>(base: &Path, subdir: &str, id: &str, m: &T) -> std::io::Result<()> {
    let d = dir(base, subdir);
    std::fs::create_dir_all(&d)?;
    let json = serde_json::to_string_pretty(m).unwrap_or_else(|_| "{}".into());
    std::fs::write(d.join(format!("{}.json", sanitize(id))), json)
}

pub fn delete_one(base: &Path, subdir: &str, id: &str) -> std::io::Result<()> {
    let path = dir(base, subdir).join(format!("{}.json", sanitize(id)));
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
