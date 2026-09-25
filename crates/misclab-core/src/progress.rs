//! Per-node progress reporting, decoupled from Tauri. The engine emits
//! node-scoped [`ProgressEvent`]s into a [`ProgressSink`]; `src-tauri` supplies a
//! Channel-backed implementation keyed by node id, while tests use [`NullSink`].

use crate::node::PortMap;

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Events streamed as a graph (or a single node) executes.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    NodeEntered {
        node: String,
    },
    NodeProgress {
        node: String,
        pct: f32,
    },
    /// `outputs` carries the node's results (top-level graph only, so the UI can
    /// show each node's output as soon as it finishes); `cached` = served from the
    /// run cache without executing.
    NodeDone {
        node: String,
        outputs: Option<PortMap>,
        cached: bool,
    },
    /// Not executed because a required input's upstream failed or was skipped.
    NodeSkipped {
        node: String,
        reason: String,
    },
    NodeFailed {
        node: String,
        error: String,
    },
    Log {
        node: Option<String>,
        level: LogLevel,
        message: String,
    },
}

pub trait ProgressSink: Send + Sync {
    fn emit(&self, event: ProgressEvent);
}

/// A sink that discards everything — for tests and headless runs.
pub struct NullSink;

impl ProgressSink for NullSink {
    fn emit(&self, _event: ProgressEvent) {}
}
