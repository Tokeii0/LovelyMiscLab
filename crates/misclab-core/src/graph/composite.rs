//! User-defined **composite modules**: a saved sub-graph that appears in the
//! palette as a single node. At execution a [`SubgraphNode`] runs the inner
//! graph via a nested [`GraphExecutor`], mapping the module's boundary inputs
//! into the sub-graph and its boundary outputs back out.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::graph::executor::GraphExecutor;
use crate::graph::model::SerializedGraph;
use crate::graph::port::{PortType, PortValue};
use crate::node::descriptor::{Cost, NodeDescriptor, PortSpec};
use crate::node::registry::{NodeFactory, NodeRegistry};
use crate::node::{Node, NodeCtx, PortMap};

/// Guards against a composite that (transitively) references itself.
const MAX_DEPTH: usize = 16;

/// One externally-visible port on a module, bound to an inner node's port.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundaryPort {
    /// External port name shown on the module node.
    pub name: String,
    pub label: String,
    /// Resolved from the inner port's declared type.
    pub port_type: PortType,
    /// Inner node id this boundary maps to.
    pub node: String,
    /// Inner node's port name.
    pub port: String,
}

/// A saved sub-graph packaged as a reusable node. Persisted verbatim as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeModule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub description: String,
    pub graph: SerializedGraph,
    pub inputs: Vec<BoundaryPort>,
    pub outputs: Vec<BoundaryPort>,
}

impl CompositeModule {
    /// The palette/descriptor view of this module (ports from boundaries, no params).
    pub fn descriptor(&self) -> NodeDescriptor {
        let ports = |b: &BoundaryPort| PortSpec::new(&b.name, &b.label, b.port_type, false);
        NodeDescriptor {
            id: self.id.clone(),
            category: if self.category.is_empty() { "自定义".to_string() } else { self.category.clone() },
            display_name: self.name.clone(),
            description: self.description.clone(),
            color: if self.color.is_empty() { "#8b5cf6".to_string() } else { self.color.clone() },
            inputs: self.inputs.iter().map(ports).collect(),
            outputs: self.outputs.iter().map(ports).collect(),
            params: vec![],
            cost: Cost::Medium,
        }
    }

    /// A factory that builds a fresh [`SubgraphNode`] for this module.
    pub fn factory(&self) -> NodeFactory {
        let graph = self.graph.clone();
        let inputs = self.inputs.clone();
        let outputs = self.outputs.clone();
        Arc::new(move || {
            Arc::new(SubgraphNode {
                graph: graph.clone(),
                inputs: inputs.clone(),
                outputs: outputs.clone(),
            })
        })
    }
}

/// Register a set of composite modules on top of a base registry (builtins),
/// returning the combined registry that the app and nested execution both use.
pub fn registry_with(base: &NodeRegistry, modules: &[CompositeModule]) -> NodeRegistry {
    let mut reg = base.clone();
    for m in modules {
        reg.register(m.descriptor(), m.factory());
    }
    reg
}

/// The node that runs a composite module's inner sub-graph.
pub struct SubgraphNode {
    graph: SerializedGraph,
    inputs: Vec<BoundaryPort>,
    outputs: Vec<BoundaryPort>,
}

impl Node for SubgraphNode {
    fn run(&self, inputs: &PortMap, _params: &serde_json::Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        if ctx.depth > MAX_DEPTH {
            return Err(CoreError::Graph("模块嵌套过深（可能存在自引用）".into()));
        }
        // Map the module's incoming values onto its inner boundary input ports.
        let mut seed: HashMap<(String, String), PortValue> = HashMap::new();
        for bp in &self.inputs {
            if let Some(v) = inputs.get(&bp.name) {
                if !matches!(v, PortValue::None) {
                    seed.insert((bp.node.clone(), bp.port.clone()), v.clone());
                }
            }
        }
        let outs = GraphExecutor::new(ctx.registry, &self.graph)?
            .with_env(ctx.env.clone())
            .with_seed_inputs(seed)
            .with_depth(ctx.depth + 1)
            .run(ctx.sink, ctx.cancel)?;
        // Collect the boundary outputs into the module's output map.
        let mut result = PortMap::new();
        for bp in &self.outputs {
            if let Some(pm) = outs.get(&bp.node) {
                if let Some(v) = pm.get(&bp.port) {
                    result.insert(bp.name.clone(), v.clone());
                }
            }
        }
        Ok(result)
    }
}
