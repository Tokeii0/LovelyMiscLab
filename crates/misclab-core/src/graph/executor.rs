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

use serde_json::{json, Value};

use crate::cancel::CancellationToken;
use crate::error::CoreError;
use crate::graph::compute::ComputeGraph;
use crate::graph::model::{NodeId, NodeInstance, SerializedGraph};
use crate::graph::port::PortValue;
use crate::node::descriptor::ParamWidget;
use crate::node::registry::NodeRegistry;
use crate::node::{NodeCtx, PortMap};
use crate::progress::{ProgressEvent, ProgressSink};

/// Coerce a connected port value into the JSON shape a param widget expects.
/// This is what lets any parameter be driven by an upstream node ("convert to input").
fn coerce_param(value: &PortValue, widget: &ParamWidget) -> Value {
    match widget {
        ParamWidget::Number { .. } | ParamWidget::Slider { .. } => match value {
            PortValue::Number(n) => json!(n),
            PortValue::Bool(b) => json!(if *b { 1.0 } else { 0.0 }),
            PortValue::Text(t) => t.trim().parse::<f64>().map(|n| json!(n)).unwrap_or(Value::Null),
            _ => Value::Null,
        },
        ParamWidget::Toggle => match value {
            PortValue::Bool(b) => json!(b),
            PortValue::Number(n) => json!(*n != 0.0),
            PortValue::Text(t) => json!(matches!(
                t.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on" | "是"
            )),
            _ => Value::Null,
        },
        // Text / Select / File → string
        _ => match value {
            PortValue::Text(t) => json!(t),
            PortValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    json!((*n as i64).to_string())
                } else {
                    json!(n.to_string())
                }
            }
            PortValue::Bool(b) => json!(b.to_string()),
            PortValue::StringList(v) => json!(v.join("\n")),
            _ => json!(""),
        },
    }
}

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
    /// Externally-seeded inputs keyed by (node id, port). Used to feed a composite
    /// module's boundary inputs into the dangling ports of its inner sub-graph.
    seed_inputs: HashMap<(NodeId, String), PortValue>,
    /// Nesting depth (0 at the top level), incremented for each nested sub-graph.
    depth: usize,
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
            seed_inputs: HashMap::new(),
            depth: 0,
        })
    }

    /// Attach the runtime environment (AI config, default output dir).
    pub fn with_env(mut self, env: crate::node::NodeEnv) -> Self {
        self.env = env;
        self
    }

    /// Pre-seed inputs on specific `(node, port)` slots (composite boundary inputs).
    pub fn with_seed_inputs(mut self, seed: HashMap<(NodeId, String), PortValue>) -> Self {
        self.seed_inputs = seed;
        self
    }

    /// Set the sub-graph nesting depth (composite recursion guard).
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
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
            let params = self.effective_params(&inst, &inputs);
            let key = cache_key(&inst.descriptor_id, &params, &inputs);

            if let Some(cached) = cache.get(&key) {
                outputs.insert(node_id.clone(), cached.clone());
                sink.emit(ProgressEvent::NodeDone { node: node_id });
                continue;
            }

            match self.run_one(&inst, &inputs, &params, sink, cancel) {
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
    /// Ports left dangling by edges fall back to any externally-seeded value.
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
        // Edges win; seeded inputs only fill ports no edge provided.
        for ((n, port), val) in &self.seed_inputs {
            if n == node_id {
                inputs.entry(port.clone()).or_insert_with(|| val.clone());
            }
        }
        inputs
    }

    /// Merge param-targeting input edges into a node's params. An edge whose target
    /// port matches a *param* name (not a declared input port) overrides that param
    /// with the connected, coerced value — this is "convert parameter to input".
    fn effective_params(&self, inst: &NodeInstance, gathered: &PortMap) -> Value {
        let mut params = inst.params.clone();
        if !params.is_object() {
            params = Value::Object(serde_json::Map::new());
        }
        let Some(entry) = self.registry.get(&inst.descriptor_id) else {
            return params;
        };
        let desc = &entry.descriptor;
        let obj = params.as_object_mut().expect("params ensured to be an object");
        for spec in &desc.params {
            // A declared input port with the same name wins over param injection.
            if desc.inputs.iter().any(|p| p.name == spec.name) {
                continue;
            }
            if let Some(val) = gathered.get(&spec.name) {
                if !matches!(val, PortValue::None) {
                    obj.insert(spec.name.clone(), coerce_param(val, &spec.widget));
                }
            }
        }
        params
    }

    fn run_one(
        &self,
        inst: &NodeInstance,
        inputs: &PortMap,
        params: &Value,
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
            registry: self.registry,
            depth: self.depth,
        };
        node.run(inputs, params, &mut ctx)
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
            registry,
            depth: 0,
        };
        node.run(inputs, params, &mut ctx)
    }
}
