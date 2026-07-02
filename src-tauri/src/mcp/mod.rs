//! Embedded MCP (Model Context Protocol) server — lets external AI clients drive
//! the node-graph engine over a bearer-gated localhost HTTP endpoint.
//!
//! Architecture: the server runs on its **own** multi-thread tokio runtime in a
//! dedicated thread (decoupled from Tauri's runtime, which may not expose the
//! `net` feature axum needs). A [`McpServerHandle`] owns the shutdown token and
//! the thread join handle.

pub mod auth;
pub mod handlers;
pub mod io_adapt;
pub mod server;
pub mod state;

#[cfg(test)]
mod itest;

pub use state::McpState;

use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

/// Handle to a running embedded MCP server. Dropping it does not stop the
/// server — call [`McpServerHandle::stop`] for a graceful shutdown.
pub struct McpServerHandle {
    cancel: CancellationToken,
    join: Option<JoinHandle<()>>,
    pub addr: SocketAddr,
}

impl McpServerHandle {
    /// Signal graceful shutdown and wait for the server thread to unwind.
    pub fn stop(mut self) {
        self.cancel.cancel();
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Start the MCP server on its own runtime thread, bound to `addr`, sharing
/// `state`.
///
/// Blocks briefly to confirm the bind succeeded so a busy port surfaces as an
/// `Err` (and, via the Settings UI, to the user) instead of failing silently.
pub fn start(state: McpState, addr: SocketAddr) -> std::io::Result<McpServerHandle> {
    let cancel = CancellationToken::new();
    let child = cancel.clone();
    // Reports the bind result (Ok once bound, Err if bind failed).
    let (tx, rx) = mpsc::channel::<std::io::Result<()>>();

    let join = std::thread::Builder::new()
        .name("mcp-server".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            rt.block_on(async move {
                let listener = match server::bind(addr).await {
                    Ok(l) => {
                        let _ = tx.send(Ok(()));
                        l
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                server::serve(state, listener, child).await;
            });
        })?;

    // Once bound, `serve` blocks until shutdown, so no further message arrives —
    // a timeout therefore means "bound and running".
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(McpServerHandle {
            cancel,
            join: Some(join),
            addr,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Ok(McpServerHandle {
            cancel,
            join: Some(join),
            addr,
        }),
    }
}
