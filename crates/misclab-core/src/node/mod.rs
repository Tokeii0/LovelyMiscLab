//! The `Node` trait, execution context, and the descriptor/registry submodules.
//!
//! A [`Node`] is a pure operation: `run(inputs, params) -> outputs`. The same
//! method powers both graph execution and standalone "quick tool" use.

pub mod descriptor;
pub mod registry;

use std::collections::HashMap;

use crate::cancel::CancellationToken;
use crate::error::CoreError;
use crate::graph::port::PortValue;
use crate::progress::{LogLevel, ProgressEvent, ProgressSink};

/// A node's inputs or outputs: port name → value.
pub type PortMap = HashMap<String, PortValue>;

/// Runtime environment injected into every node (AI config + default output dir).
/// Also serves as the persisted app settings.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEnv {
    pub ai: crate::ai::AiConfig,
    pub output_dir: String,
    /// External tool paths (python, tshark, 7z, exiftool, java, …). Consumed by
    /// future external-tool nodes; configured in Settings.
    #[serde(default)]
    pub tools: std::collections::HashMap<String, String>,
}

/// Everything a node needs while running. The progress helpers stamp the node id
/// so the frontend can attribute updates to the right node on the canvas.
pub struct NodeCtx<'a> {
    pub node_id: String,
    pub sink: &'a dyn ProgressSink,
    pub cancel: &'a CancellationToken,
    pub env: &'a NodeEnv,
    /// The active registry, so a composite/sub-graph node can run its inner graph
    /// with the same node set (including other composites).
    pub registry: &'a registry::NodeRegistry,
    /// Nesting depth of sub-graph execution; guards against self-referential
    /// composite modules recursing forever.
    pub depth: usize,
}

impl NodeCtx<'_> {
    /// Report fractional progress (0.0..=1.0) for this node.
    pub fn progress(&self, pct: f32) {
        self.sink.emit(ProgressEvent::NodeProgress {
            node: self.node_id.clone(),
            pct,
        });
    }

    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        self.sink.emit(ProgressEvent::Log {
            node: Some(self.node_id.clone()),
            level,
            message: message.into(),
        });
    }

    pub fn check_cancel(&self) -> Result<(), CoreError> {
        self.cancel.check()
    }
}

/// A node type. Implement this + a [`descriptor::NodeDescriptor`] + register it,
/// and the node becomes available on the canvas and as a quick tool.
pub trait Node: Send + Sync {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError>;
}
