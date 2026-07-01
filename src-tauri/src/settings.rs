//! Persistence for app settings (AI model config + default output dir) in a JSON
//! file under the app data dir.

use std::path::Path;

use misclab_core::node::NodeEnv;

const FILE: &str = "settings.json";

pub fn load(dir: &Path) -> NodeEnv {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, settings: &NodeEnv) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(settings).unwrap_or_else(|_| "{}".into());
    std::fs::write(dir.join(FILE), json)
}
