//! User-defined composite (sub-graph) module commands: list / save / delete.
//! Modules are persisted as JSON and merged into the effective registry on demand
//! (see `commands::graph::combined_registry`).

use tauri::{Manager, State};

use misclab_core::graph::composite::CompositeModule;

use crate::error::AppError;
use crate::state::AppState;

fn data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    app.path().app_data_dir().map_err(|e| AppError::new("path", e.to_string()))
}

#[tauri::command]
pub fn list_composite_modules(state: State<'_, AppState>) -> Vec<CompositeModule> {
    state.composites.lock().expect("composites mutex poisoned").clone()
}

#[tauri::command]
pub fn save_composite_module(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    module: CompositeModule,
) -> Result<(), AppError> {
    let dir = data_dir(&app)?;
    crate::modules::save_one(&dir, "modules", &module.id, &module).map_err(|e| AppError::new("io", e.to_string()))?;
    let mut comps = state.composites.lock().expect("composites mutex poisoned");
    comps.retain(|m| m.id != module.id); // upsert by id
    comps.push(module);
    comps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(())
}

#[tauri::command]
pub fn delete_composite_module(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let dir = data_dir(&app)?;
    crate::modules::delete_one(&dir, "modules", &id).map_err(|e| AppError::new("io", e.to_string()))?;
    state.composites.lock().expect("composites mutex poisoned").retain(|m| m.id != id);
    Ok(())
}
