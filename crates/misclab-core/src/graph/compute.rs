//! The petgraph-backed compute graph: builds from a [`SerializedGraph`], rejects
//! cycles, and yields a topological execution order.

use std::collections::HashMap;

use petgraph::algo::toposort;
use petgraph::graph::DiGraph;

use crate::error::CoreError;
use crate::graph::model::{Edge, NodeId, SerializedGraph};

pub struct ComputeGraph {
    graph: DiGraph<NodeId, ()>,
    /// The original edges, retained for input-gathering during execution.
    pub edges: Vec<Edge>,
}

impl ComputeGraph {
    pub fn from_serialized(g: &SerializedGraph) -> Result<Self, CoreError> {
        let mut graph = DiGraph::new();
        let mut index = HashMap::new();

        for n in &g.nodes {
            let idx = graph.add_node(n.id.clone());
            index.insert(n.id.clone(), idx);
        }

        for e in &g.edges {
            let from = *index.get(&e.from.node).ok_or_else(|| {
                CoreError::Graph(format!("edge references unknown node '{}'", e.from.node))
            })?;
            let to = *index.get(&e.to.node).ok_or_else(|| {
                CoreError::Graph(format!("edge references unknown node '{}'", e.to.node))
            })?;
            graph.add_edge(from, to, ());
        }

        let cg = Self {
            graph,
            edges: g.edges.clone(),
        };
        // Validate acyclicity up front.
        cg.execution_order()?;
        Ok(cg)
    }

    /// Node ids in dependency (topological) order.
    pub fn execution_order(&self) -> Result<Vec<NodeId>, CoreError> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order.into_iter().map(|i| self.graph[i].clone()).collect()),
            Err(_) => Err(CoreError::Graph("graph contains a cycle".into())),
        }
    }
}
