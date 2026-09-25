//! Save / open a flow project as a JSON file at a user-chosen path (the path is
//! obtained on the frontend via the dialog plugin).

use std::path::PathBuf;

use crate::commands::blocking;
use crate::error::AppError;

#[tauri::command]
pub async fn save_project(path: String, contents: String) -> Result<(), AppError> {
    blocking(move || {
        crate::settings::write_atomic(&PathBuf::from(path), contents.as_bytes())
            .map_err(|e| AppError::new("io", e.to_string()))
    })
    .await
}

#[tauri::command]
pub async fn load_project(path: String) -> Result<String, AppError> {
    blocking(move || std::fs::read_to_string(&path).map_err(|e| AppError::new("io", e.to_string())))
        .await
}
