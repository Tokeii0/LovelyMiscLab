//! System / health commands. These prove the frontend↔Rust round-trip and that
//! the database initialized, satisfying the M0 milestone.

use serde::Serialize;

use crate::error::AppResult;
use crate::state::AppState;

/// Round-trip smoke test.
#[tauri::command]
pub fn ping(name: String) -> String {
    format!("pong: {name}")
}

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

/// Proves the database is initialized and queryable (counts projects).
#[tauri::command]
pub fn db_health(state: tauri::State<'_, AppState>) -> AppResult<u32> {
    let conn = state.db.conn();
    let n: u32 = conn.query_row("SELECT count(*) FROM projects", [], |r| r.get(0))?;
    Ok(n)
}
