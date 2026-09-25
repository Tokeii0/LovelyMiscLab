//! `#[tauri::command]` surface. Grouped by concern.

pub mod agent;
pub mod ai_common;
pub mod ai_workflow;
pub mod galgame;
pub mod graph;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod modules;
pub mod project;
pub mod script_modules;
pub mod settings;
pub mod system;
pub mod update;

use crate::error::AppError;

/// Run blocking work (file I/O, process spawns…) on the blocking pool. Tauri
/// runs plain `fn` commands on the main thread, where slow I/O freezes the UI.
pub(crate) async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::new("join", e.to_string()))?
}
