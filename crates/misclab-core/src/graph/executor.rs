//! The graph executor. Runs nodes in topological order, feeding each node's
//! outputs to downstream inputs, emitting per-node progress. A node failure is
//! non-fatal: it is reported and execution continues (downstream nodes that
//! depended on it simply won't receive that input).
//!
//! The same underlying `Node::run` powers standalone single-node execution via
//! [`GraphExecutor::run_node`].

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use serde_json::Value;

use crate::cancel::CancellationToken;
use crate::error::CoreError;
use crate::graph::compute::ComputeGraph;
use crate::graph::model::{NodeId, NodeInstance, SerializedGraph};
use crate::node::registry::NodeRegistry;
use crate::node::{NodeCtx, PortMap};
use crate::progress::{ProgressEvent, ProgressSink};

/// Per-node output maps produced by a graph run.
pub type GraphOutputs = HashMap<NodeId, PortMap>;

/// Content-keyed cache of node outputs. Persisting it across runs lets an added
/// or edited node recompute incrementally while unchanged nodes are reused.
pub type NodeCache = HashMap<u64, PortMap>;

/// A stable (within-process) hash of everything that determines a node's output:
/// its descriptor, params, and the values on its input ports.
fn cache_key(descriptor_id: &str, params: &Value, inputs: &PortMap) -> u64 {
    let mut hasher = DefaultHasher::new();
    descriptor_id.hash(&mut hasher);
    params.to_string().hash(&mut hasher);
    let mut names: Vec<&String> = inputs.keys().collect();
    names.sort();
    for name in names {
        name.hash(&mut hasher);
        serde_json::to_string(&inputs[name])
            .unwrap_or_default()
            .hash(&mut hasher);
    }
    hasher.finish()
}

pub struct GraphExecutor<'a> {
    registry: &'a NodeRegistry,
    compute: ComputeGraph,
    nodes: HashMap<NodeId, NodeInstance>,
    env: crate::node::NodeEnv,
}

impl<'a> GraphExecutor<'a> {
    pub fn new(registry: &'a NodeRegistry, graph: &SerializedGraph) -> Result<Self, CoreError> {
        let compute = ComputeGraph::from_serialized(graph)?;
        let nodes = graph
            .nodes
            .iter()
            .map(|n| (n.id.clone(), n.clone()))
            .collect();
        Ok(Self {
            registry,
            compute,
            nodes,
            env: crate::node::NodeEnv::default(),
        })
    }

    /// Attach the runtime environment (AI config, default output dir).
    pub fn with_env(mut self, env: crate::node::NodeEnv) -> Self {
        self.env = env;
        self
    }

    /// Execute the whole graph (no cross-run cache).
    pub fn run(
        &self,
        sink: &dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Result<GraphOutputs, CoreError> {
        let mut cache = NodeCache::new();
        self.run_with_cache(sink, cancel, &mut cache)
    }

    /// Execute the graph, reusing cached outputs for nodes whose descriptor,
    /// params, and inputs are unchanged. This powers "live mode": adding one node
    /// recomputes only that node while the rest are served from cache.
    pub fn run_with_cache(
        &self,
        sink: &dyn ProgressSink,
        cancel: &CancellationToken,
        cache: &mut NodeCache,
    ) -> Result<GraphOutputs, CoreError> {
        let order = self.compute.execution_order()?;
        let mut outputs: GraphOutputs = HashMap::new();

        for node_id in order {
            cancel.check()?;
            let inst = self
                .nodes
                .get(&node_id)
                .expect("execution order only contains known nodes")
                .clone();

            sink.emit(ProgressEvent::NodeEntered {
                node: node_id.clone(),
            });

            let inputs = self.gather_inputs(&node_id, &outputs);
            let key = cache_key(&inst.descriptor_id, &inst.params, &inputs);

            if let Some(cached) = cache.get(&key) {
                outputs.insert(node_id.clone(), cached.clone());
                sink.emit(ProgressEvent::NodeDone { node: node_id });
                continue;
            }

            match self.run_one(&inst, &inputs, sink, cancel) {
                Ok(out) => {
                    if cache.len() >= 4096 {
                        cache.clear();
                    }
                    cache.insert(key, out.clone());
                    outputs.insert(node_id.clone(), out);
                    sink.emit(ProgressEvent::NodeDone { node: node_id });
                }
                Err(e) => {
                    sink.emit(ProgressEvent::NodeFailed {
                        node: node_id,
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(outputs)
    }

    /// Collect a node's inputs by reading upstream outputs along incoming edges.
    fn gather_inputs(&self, node_id: &NodeId, outputs: &GraphOutputs) -> PortMap {
        let mut inputs = PortMap::new();
        for edge in &self.compute.edges {
            if &edge.to.node != node_id {
                continue;
            }
            if let Some(upstream) = outputs.get(&edge.from.node) {
                if let Some(value) = upstream.get(&edge.from.port) {
                    inputs.insert(edge.to.port.clone(), value.clone());
                }
            }
        }
        inputs
    }

    fn run_one(
        &self,
        inst: &NodeInstance,
        inputs: &PortMap,
        sink: &dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Result<PortMap, CoreError> {
        let node = self
            .registry
            .create(&inst.descriptor_id)
            .ok_or_else(|| CoreError::NodeNotFound(inst.descriptor_id.clone()))?;
        let mut ctx = NodeCtx {
            node_id: inst.id.clone(),
            sink,
            cancel,
            env: &self.env,
        };
        node.run(inputs, &inst.params, &mut ctx)
    }

    /// Run a single node standalone (the "quick tool" path) with explicit inputs
    /// and params. Uses the exact same `Node::run` as graph execution.
    pub fn run_node(
        registry: &NodeRegistry,
        descriptor_id: &str,
        inputs: &PortMap,
        params: &Value,
        sink: &dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Result<PortMap, CoreError> {
        let env = crate::node::NodeEnv::default();
        Self::run_node_with_env(registry, descriptor_id, inputs, params, &env, sink, cancel)
    }

    /// Standalone single-node run with an explicit runtime environment.
    pub fn run_node_with_env(
        registry: &NodeRegistry,
        descriptor_id: &str,
        inputs: &PortMap,
        params: &Value,
        env: &crate::node::NodeEnv,
        sink: &dyn ProgressSink,
        cancel: &CancellationToken,
    ) -> Result<PortMap, CoreError> {
        let node = registry
            .create(descriptor_id)
            .ok_or_else(|| CoreError::NodeNotFound(descriptor_id.to_string()))?;
        let mut ctx = NodeCtx {
            node_id: format!("standalone:{descriptor_id}"),
            sink,
            cancel,
            env,
        };
        node.run(inputs, params, &mut ctx)
    }
}
