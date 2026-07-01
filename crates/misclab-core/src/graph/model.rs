//! The serialized graph — what the frontend saves/loads and sends to execute.

use serde::{Deserialize, Serialize};

pub type NodeId = String;

/// A reference to a specific port on a specific node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortRef {
    pub node: NodeId,
    pub port: String,
}

/// A connection: `from` (an output port) → `to` (an input port).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from: PortRef,
    pub to: PortRef,
}

/// One placed node: which descriptor it is, its params, and canvas position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInstance {
    pub id: NodeId,
    pub descriptor_id: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub position: (f32, f32),
}

/// The whole graph, as persisted and executed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedGraph {
    pub nodes: Vec<NodeInstance>,
    pub edges: Vec<Edge>,
}
