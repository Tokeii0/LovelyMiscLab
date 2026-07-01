//! "Convert parameter to input" — an edge targeting a param name overrides that
//! param with the connected (coerced) value, so any param can be node-driven.

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::model::{Edge, NodeInstance, PortRef, SerializedGraph};
use misclab_core::graph::port::PortValue;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::json;

fn node(id: &str, descriptor: &str, params: serde_json::Value) -> NodeInstance {
    NodeInstance {
        id: id.into(),
        descriptor_id: descriptor.into(),
        params,
        position: (0.0, 0.0),
    }
}
fn edge(fnode: &str, fport: &str, tnode: &str, tport: &str) -> Edge {
    Edge {
        from: PortRef { node: fnode.into(), port: fport.into() },
        to: PortRef { node: tnode.into(), port: tport.into() },
    }
}

#[test]
fn text_param_driven_by_input_overrides_static() {
    // hash's static algorithm is MD5, but an edge feeds "SHA256" into the param.
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![
            node("data", "text_input", json!({ "text": "hello" })),
            node("algo", "text_input", json!({ "text": "SHA256" })),
            node("h", "hash", json!({ "algorithm": "MD5" })),
        ],
        edges: vec![
            edge("data", "text", "h", "data"),
            edge("algo", "text", "h", "algorithm"),
        ],
    };
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    match out.get("h").and_then(|m| m.get("text")) {
        // SHA256("hello"), not MD5.
        Some(PortValue::Text(s)) => {
            assert_eq!(s, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn number_param_coerced_from_text_input() {
    // range.end static is 10, but a text "3" is coerced to the number 3.
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![
            node("n", "text_input", json!({ "text": "3" })),
            node("r", "range", json!({ "start": 0, "end": 10, "step": 1 })),
        ],
        edges: vec![edge("n", "text", "r", "end")],
    };
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    match out.get("r").and_then(|m| m.get("list")) {
        Some(PortValue::StringList(v)) => assert_eq!(v, &vec!["0", "1", "2"]),
        other => panic!("got {other:?}"),
    }
}
