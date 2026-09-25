//! System info command (app + engine version for the settings/about screen).

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub core_version: String,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "LovelyMiscLab".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        core_version: misclab_core::core_version().into(),
    }
}
