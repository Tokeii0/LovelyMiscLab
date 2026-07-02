//! Commands for controlling the embedded MCP server (start/stop/status) and
//! reading/writing its persisted config. The Settings UI drives these.

use tauri::{Manager, State};

use crate::error::AppError;
use crate::mcp::state::{load_config, save_config, CanvasSnapshot, McpSettings, McpState};
use crate::state::AppState;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
    pub bind_all: bool,
    pub endpoint: String,
}

fn data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::new("path", e.to_string()))
}

fn endpoint(cfg: &McpSettings) -> String {
    let host = if cfg.bind_all { "0.0.0.0" } else { "127.0.0.1" };
    format!("http://{host}:{}/mcp", cfg.port)
}

fn status(running: bool, cfg: &McpSettings) -> McpStatus {
    McpStatus {
        running,
        port: cfg.port,
        bind_all: cfg.bind_all,
        endpoint: endpoint(cfg),
    }
}

/// Current config (includes the token — this is the local GUI, not the MCP tool
/// surface, so the user can view/copy it).
#[tauri::command]
pub fn mcp_get_config(app: tauri::AppHandle) -> Result<McpSettings, AppError> {
    Ok(load_config(&data_dir(&app)?))
}

/// Persist config. Does not start/stop a running server (call mcp_start/stop).
#[tauri::command]
pub fn mcp_set_config(app: tauri::AppHandle, config: McpSettings) -> Result<(), AppError> {
    save_config(&data_dir(&app)?, &config).map_err(AppError::from)
}

/// Frontend → backend canvas mirror. Called (debounced) whenever the React Flow
/// store changes so `get_canvas` reflects what the user sees. Keeps `rev`
/// monotonic to help break the echo loop.
#[tauri::command]
pub fn sync_canvas(state: State<'_, AppState>, snapshot: CanvasSnapshot) {
    let mut cv = state.canvas.lock().expect("canvas mutex poisoned");
    let rev = snapshot.rev.max(cv.rev);
    *cv = snapshot;
    cv.rev = rev;
}

#[tauri::command]
pub fn mcp_status(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<McpStatus, AppError> {
    let cfg = load_config(&data_dir(&app)?);
    let guard = state.mcp.lock().expect("mcp mutex poisoned");
    match guard.as_ref() {
        // Reflect the actually-bound address when running.
        Some(h) => Ok(McpStatus {
            running: true,
            port: h.addr.port(),
            bind_all: cfg.bind_all,
            endpoint: format!("http://{}/mcp", h.addr),
        }),
        None => Ok(status(false, &cfg)),
    }
}

#[tauri::command]
pub fn mcp_start(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<McpStatus, AppError> {
    let dir = data_dir(&app)?;
    let mut cfg = load_config(&dir);
    // Ensure a bearer token exists, then persist enabled=true + the token.
    if cfg.token.as_deref().unwrap_or("").is_empty() {
        cfg.token = Some(uuid::Uuid::new_v4().simple().to_string());
    }
    cfg.enabled = true;
    save_config(&dir, &cfg).ok();

    let mut guard = state.mcp.lock().expect("mcp mutex poisoned");
    if guard.is_some() {
        return Ok(status(true, &cfg));
    }
    let mcp_state = McpState::from_app(state.inner(), app.clone(), cfg.token.clone());
    let host = if cfg.bind_all { [0, 0, 0, 0] } else { [127, 0, 0, 1] };
    let addr = std::net::SocketAddr::from((host, cfg.port));
    let handle = crate::mcp::start(mcp_state, addr).map_err(AppError::from)?;
    *guard = Some(handle);
    Ok(status(true, &cfg))
}

#[tauri::command]
pub fn mcp_stop(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<McpStatus, AppError> {
    if let Some(handle) = state.mcp.lock().expect("mcp mutex poisoned").take() {
        handle.stop();
    }
    let dir = data_dir(&app)?;
    let mut cfg = load_config(&dir);
    cfg.enabled = false;
    save_config(&dir, &cfg).ok();
    Ok(status(false, &cfg))
}
