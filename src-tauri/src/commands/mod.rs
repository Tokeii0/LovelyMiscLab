//! `#[tauri::command]` surface. Grouped by concern.

pub mod ai_workflow;
pub mod graph;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod modules;
pub mod project;
pub mod script_modules;
pub mod settings;
pub mod system;
