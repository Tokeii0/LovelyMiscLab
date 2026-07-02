//! User-defined script/program node commands: list / save / delete.
//! Persisted as JSON under `<app_data_dir>/script_modules/` and merged into the
//! effective registry on demand (see `commands::graph::combined_registry`).

use tauri::{Manager, State};

use misclab_core::graph::script_node::ScriptModule;

use crate::error::AppError;
use crate::state::AppState;

const SUBDIR: &str = "script_modules";

fn data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    app.path().app_data_dir().map_err(|e| AppError::new("path", e.to_string()))
}

#[tauri::command]
pub fn list_script_modules(state: State<'_, AppState>) -> Vec<ScriptModule> {
    state.scripts.lock().expect("scripts mutex poisoned").clone()
}

#[tauri::command]
pub fn save_script_module(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    module: ScriptModule,
) -> Result<(), AppError> {
    let dir = data_dir(&app)?;
    crate::modules::save_one(&dir, SUBDIR, &module.id, &module).map_err(|e| AppError::new("io", e.to_string()))?;
    let mut scripts = state.scripts.lock().expect("scripts mutex poisoned");
    scripts.retain(|m| m.id != module.id); // upsert by id
    scripts.push(module);
    scripts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(())
}

#[tauri::command]
pub fn delete_script_module(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let dir = data_dir(&app)?;
    crate::modules::delete_one(&dir, SUBDIR, &id).map_err(|e| AppError::new("io", e.to_string()))?;
    state.scripts.lock().expect("scripts mutex poisoned").retain(|m| m.id != id);
    Ok(())
}
