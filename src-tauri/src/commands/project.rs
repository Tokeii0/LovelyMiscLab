//! Save / open a flow project as a JSON file at a user-chosen path (the path is
//! obtained on the frontend via the dialog plugin).

use crate::error::AppError;

#[tauri::command]
pub fn save_project(path: String, contents: String) -> Result<(), AppError> {
    std::fs::write(&path, contents).map_err(|e| AppError::new("io", e.to_string()))
}

#[tauri::command]
pub fn load_project(path: String) -> Result<String, AppError> {
    std::fs::read_to_string(&path).map_err(|e| AppError::new("io", e.to_string()))
}
