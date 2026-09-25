//! User-defined composite (sub-graph) module commands: list / save / delete.
//! Modules are persisted as JSON and merged into the effective registry on demand
//! (see `AppState::user_registry`).

use tauri::{Manager, State};

use misclab_core::graph::composite::CompositeModule;

use crate::commands::blocking;
use crate::error::AppError;
use crate::state::AppState;

fn data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::new("path", e.to_string()))
}

#[tauri::command]
pub fn list_composite_modules(state: State<'_, AppState>) -> Vec<CompositeModule> {
    state
        .composites
        .lock()
        .expect("composites mutex poisoned")
        .clone()
}

#[tauri::command]
pub async fn save_composite_module(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    module: CompositeModule,
) -> Result<(), AppError> {
    let dir = data_dir(&app)?;
    let copy = module.clone();
    blocking(move || {
        crate::modules::save_one(&dir, "modules", &copy.id, &copy)
            .map_err(|e| AppError::new("io", e.to_string()))
    })
    .await?;
    let mut comps = state.composites.lock().expect("composites mutex poisoned");
    comps.retain(|m| m.id != module.id); // upsert by id
    comps.push(module);
    comps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(())
}

#[tauri::command]
pub async fn delete_composite_module(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let dir = data_dir(&app)?;
    let target = id.clone();
    blocking(move || {
        crate::modules::delete_one(&dir, "modules", &target)
            .map_err(|e| AppError::new("io", e.to_string()))
    })
    .await?;
    state
        .composites
        .lock()
        .expect("composites mutex poisoned")
        .retain(|m| m.id != id);
    Ok(())
}
