//! Composite (user-defined sub-graph) module execution.

use std::collections::HashMap;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::composite::{registry_with, BoundaryPort, CompositeModule};
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::model::{NodeInstance, SerializedGraph};
use misclab_core::graph::port::{PortType, PortValue};
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::json;

fn text_in(name: &str, node: &str, port: &str) -> BoundaryPort {
    BoundaryPort { name: name.into(), label: name.into(), port_type: PortType::Text, node: node.into(), port: port.into() }
}

#[test]
fn composite_runs_inner_subgraph() {
    // Inner graph: a single Caesar-shift(+3) node. Module boundary: in→c1.text, out←c1.text.
    let module = CompositeModule {
        id: "mod_test_caesar".into(),
        name: "测试凯撒".into(),
        category: String::new(),
        color: String::new(),
        description: String::new(),
        graph: SerializedGraph {
            nodes: vec![NodeInstance {
                id: "c1".into(),
                descriptor_id: "caesar".into(),
                params: json!({ "amount": 3 }),
                position: (0.0, 0.0),
            }],
            edges: vec![],
        },
        inputs: vec![text_in("in", "c1", "text")],
        outputs: vec![text_in("out", "c1", "text")],
    };

    // The synthesized descriptor exposes the boundary ports.
    let desc = module.descriptor();
    assert_eq!(desc.inputs.len(), 1);
    assert_eq!(desc.outputs[0].name, "out");
    assert_eq!(desc.category, "自定义");

    let reg = registry_with(&default_registry(), std::slice::from_ref(&module));
    let mut inputs = HashMap::new();
    inputs.insert("in".to_string(), PortValue::Text("HELLO".to_string()));
    let out = GraphExecutor::run_node(&reg, "mod_test_caesar", &inputs, &json!({}), &NullSink, &CancellationToken::new()).unwrap();
    assert_eq!(out.get("out").unwrap().as_text().unwrap(), "KHOOR");
}

#[test]
fn composite_depth_guard_terminates() {
    // A module whose inner graph references itself would recurse forever without
    // the depth guard. The guard must make it terminate (not hang / stack-overflow).
    let looped = CompositeModule {
        id: "mod_loop".into(),
        name: "自引用".into(),
        category: String::new(),
        color: String::new(),
        description: String::new(),
        graph: SerializedGraph {
            nodes: vec![NodeInstance {
                id: "x".into(),
                descriptor_id: "mod_loop".into(),
                params: json!({}),
                position: (0.0, 0.0),
            }],
            edges: vec![],
        },
        inputs: vec![],
        outputs: vec![],
    };
    let reg = registry_with(&default_registry(), std::slice::from_ref(&looped));
    // Completes (thanks to the depth guard) instead of hanging.
    let out = GraphExecutor::run_node(&reg, "mod_loop", &HashMap::new(), &json!({}), &NullSink, &CancellationToken::new());
    assert!(out.is_ok());
}
