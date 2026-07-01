//! The node registry: maps a descriptor id to its descriptor + a factory that
//! builds fresh node instances. `descriptors()` feeds the frontend palette.

use std::collections::HashMap;
use std::sync::Arc;

use crate::node::descriptor::NodeDescriptor;
use crate::node::Node;

/// Builds a fresh node instance. Most nodes are stateless unit structs, so this
/// is typically `Arc::new(|| Arc::new(MyNode))`.
pub type NodeFactory = Arc<dyn Fn() -> Arc<dyn Node> + Send + Sync>;

pub struct RegistryEntry {
    pub descriptor: NodeDescriptor,
    pub factory: NodeFactory,
}

#[derive(Default, Clone)]
pub struct NodeRegistry {
    entries: HashMap<String, Arc<RegistryEntry>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, descriptor: NodeDescriptor, factory: NodeFactory) {
        let id = descriptor.id.clone();
        self.entries
            .insert(id, Arc::new(RegistryEntry { descriptor, factory }));
    }

    pub fn get(&self, id: &str) -> Option<&Arc<RegistryEntry>> {
        self.entries.get(id)
    }

    /// Instantiate a node by descriptor id.
    pub fn create(&self, id: &str) -> Option<Arc<dyn Node>> {
        self.entries.get(id).map(|e| (e.factory)())
    }

    /// All descriptors, sorted by (category, display name) for a stable palette.
    pub fn descriptors(&self) -> Vec<NodeDescriptor> {
        let mut v: Vec<NodeDescriptor> = self.entries.values().map(|e| e.descriptor.clone()).collect();
        v.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| a.display_name.cmp(&b.display_name))
        });
        v
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
