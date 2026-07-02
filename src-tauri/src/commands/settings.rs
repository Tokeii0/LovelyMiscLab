//! Settings commands — read/write the app settings (AI config + output dir +
//! tool paths) and detect external tools.

use serde::Serialize;
use tauri::{Manager, State};

use misclab_core::node::NodeEnv;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub available: bool,
    pub version: String,
}

/// Run `<path> <arg>` and report availability + first output line as the version.
/// Blocking; shared by the Tauri command and the MCP `detect_tool` tool.
pub(crate) fn detect_tool_impl(path: &str, arg: Option<String>) -> ToolStatus {
    if path.trim().is_empty() {
        return ToolStatus { available: false, version: String::new() };
    }
    let arg = arg.filter(|a| !a.is_empty()).unwrap_or_else(|| "--version".into());
    match std::process::Command::new(path).arg(&arg).output() {
        Ok(out) => {
            let mut text = String::from_utf8_lossy(&out.stdout).to_string();
            if text.trim().is_empty() {
                text = String::from_utf8_lossy(&out.stderr).to_string();
            }
            let version = text.lines().next().unwrap_or("").trim().to_string();
            ToolStatus {
                available: out.status.success() || !version.is_empty(),
                version,
            }
        }
        Err(_) => ToolStatus { available: false, version: String::new() },
    }
}

/// Run `<path> <arg>` and report availability + first output line as the version.
#[tauri::command]
pub async fn detect_tool(path: String, arg: Option<String>) -> ToolStatus {
    tauri::async_runtime::spawn_blocking(move || detect_tool_impl(&path, arg))
        .await
        .unwrap_or(ToolStatus { available: false, version: String::new() })
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> NodeEnv {
    state.settings.lock().expect("settings mutex poisoned").clone()
}

#[tauri::command]
pub fn set_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: NodeEnv,
) -> Result<(), AppError> {
    *state.settings.lock().expect("settings mutex poisoned") = settings.clone();
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::new("path", e.to_string()))?;
    std::fs::create_dir_all(&dir).ok();
    crate::settings::save(&dir, &settings).map_err(|e| AppError::new("io", e.to_string()))?;
    Ok(())
}
