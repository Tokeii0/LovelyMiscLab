//! Integration tests for the node-graph engine.

use std::collections::HashMap;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::model::{Edge, NodeInstance, PortRef, SerializedGraph};
use misclab_core::graph::port::{PortType, PortValue};
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::json;

fn node(id: &str, descriptor_id: &str, params: serde_json::Value) -> NodeInstance {
    NodeInstance {
        id: id.into(),
        descriptor_id: descriptor_id.into(),
        params,
        position: (0.0, 0.0),
    }
}

fn edge(from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> Edge {
    Edge {
        from: PortRef {
            node: from_node.into(),
            port: from_port.into(),
        },
        to: PortRef {
            node: to_node.into(),
            port: to_port.into(),
        },
    }
}

#[test]
fn runs_text_input_into_output() {
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![
            node("a", "text_input", json!({ "text": "flag{hello}" })),
            node("b", "text_output", json!({})),
        ],
        edges: vec![edge("a", "text", "b", "text")],
    };

    let exec = GraphExecutor::new(&reg, &graph).expect("graph builds");
    let outputs = exec
        .run(&NullSink, &CancellationToken::new())
        .expect("graph runs");

    match outputs.get("b").and_then(|m| m.get("value")) {
        Some(PortValue::Text(s)) => assert_eq!(s, "flag{hello}"),
        other => panic!("unexpected output: {other:?}"),
    }
}

#[test]
fn rejects_cycles() {
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![
            node("a", "text_output", json!({})),
            node("b", "text_output", json!({})),
        ],
        edges: vec![
            edge("a", "value", "b", "text"),
            edge("b", "value", "a", "text"),
        ],
    };
    assert!(GraphExecutor::new(&reg, &graph).is_err());
}

#[test]
fn rejects_edge_to_unknown_node() {
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![node("a", "text_input", json!({ "text": "x" }))],
        edges: vec![edge("a", "text", "ghost", "text")],
    };
    assert!(GraphExecutor::new(&reg, &graph).is_err());
}

#[test]
fn standalone_node_runs() {
    let reg = default_registry();
    let inputs = HashMap::new();
    let out = GraphExecutor::run_node(
        &reg,
        "text_input",
        &inputs,
        &json!({ "text": "hi" }),
        &NullSink,
        &CancellationToken::new(),
    )
    .expect("standalone runs");

    match out.get("text") {
        Some(PortValue::Text(s)) => assert_eq!(s, "hi"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn port_type_validation() {
    assert!(PortType::Text.accepts(PortType::Text));
    assert!(PortType::Any.accepts(PortType::Bytes));
    assert!(PortType::Bytes.accepts(PortType::Any));
    assert!(!PortType::Text.accepts(PortType::Number));
}

#[test]
fn descriptors_are_exported() {
    let reg = default_registry();
    let ds = reg.descriptors();
    assert!(ds.iter().any(|d| d.id == "text_input"));
    assert!(ds.iter().any(|d| d.id == "text_output"));
}
