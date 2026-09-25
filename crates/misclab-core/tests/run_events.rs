//! What a graph run reports while it executes: outputs stream with `NodeDone`,
//! cached nodes don't re-announce themselves, a failure skips its dependants,
//! and volatile nodes are never served from the cache.

use std::sync::Mutex;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::{GraphExecutor, NodeCache};
use misclab_core::graph::model::{Edge, NodeInstance, PortRef, SerializedGraph};
use misclab_core::graph::port::PortValue;
use misclab_core::nodes::default_registry;
use misclab_core::progress::{ProgressEvent, ProgressSink};
use serde_json::json;

#[derive(Default)]
struct Recorder(Mutex<Vec<ProgressEvent>>);

impl ProgressSink for Recorder {
    fn emit(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl Recorder {
    fn take(&self) -> Vec<ProgressEvent> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

fn node(id: &str, descriptor_id: &str, params: serde_json::Value) -> NodeInstance {
    NodeInstance {
        id: id.into(),
        descriptor_id: descriptor_id.into(),
        params,
        position: (0.0, 0.0),
    }
}

fn edge(from: (&str, &str), to: (&str, &str)) -> Edge {
    Edge {
        from: PortRef {
            node: from.0.into(),
            port: from.1.into(),
        },
        to: PortRef {
            node: to.0.into(),
            port: to.1.into(),
        },
    }
}

fn entered(events: &[ProgressEvent], id: &str) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ProgressEvent::NodeEntered { node } if node == id))
}

fn done(events: &[ProgressEvent], id: &str) -> Option<(bool, bool)> {
    events.iter().find_map(|e| match e {
        ProgressEvent::NodeDone {
            node,
            outputs,
            cached,
        } if node == id => Some((outputs.is_some(), *cached)),
        _ => None,
    })
}

#[test]
fn outputs_stream_with_node_done_and_cache_hits_stay_quiet() {
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![
            node("a", "text_input", json!({ "text": "68656c6c6f" })),
            node("b", "hex_decode", json!({})),
        ],
        edges: vec![edge(("a", "text"), ("b", "text"))],
    };
    let exec = GraphExecutor::new(&reg, &graph).unwrap();
    let sink = Recorder::default();
    let mut cache = NodeCache::new();

    exec.run_with_cache(&sink, &CancellationToken::new(), &mut cache)
        .unwrap();
    let first = sink.take();
    assert!(entered(&first, "b"));
    assert_eq!(done(&first, "b"), Some((true, false)));

    exec.run_with_cache(&sink, &CancellationToken::new(), &mut cache)
        .unwrap();
    let second = sink.take();
    assert!(!entered(&second, "b"), "cache hit must not re-enter");
    assert_eq!(done(&second, "b"), Some((true, true)));
}

#[test]
fn failure_skips_dependants_with_a_reason() {
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![
            node("a", "text_input", json!({ "text": "not hex!" })),
            node("b", "hex_decode", json!({})),
            node("c", "text_output", json!({})),
        ],
        edges: vec![
            edge(("a", "text"), ("b", "text")),
            edge(("b", "text"), ("c", "text")),
        ],
    };
    let sink = Recorder::default();
    GraphExecutor::new(&reg, &graph)
        .unwrap()
        .run(&sink, &CancellationToken::new())
        .unwrap();
    let events = sink.take();
    assert!(events
        .iter()
        .any(|e| matches!(e, ProgressEvent::NodeFailed { node, .. } if node == "b")));
    assert!(events.iter().any(
        |e| matches!(e, ProgressEvent::NodeSkipped { node, reason } if node == "c" && reason.contains("跳过"))
    ));
    assert!(!entered(&events, "c"));
}

#[test]
fn volatile_nodes_are_never_cached() {
    let dir = std::env::temp_dir().join(format!("misclab_volatile_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input.txt");
    std::fs::write(&path, "first").unwrap();

    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![node(
            "f",
            "file_import",
            json!({ "path": path.to_string_lossy() }),
        )],
        edges: vec![],
    };
    let exec = GraphExecutor::new(&reg, &graph).unwrap();
    let sink = Recorder::default();
    let mut cache = NodeCache::new();
    let cancel = CancellationToken::new();

    exec.run_with_cache(&sink, &cancel, &mut cache).unwrap();
    std::fs::write(&path, "second").unwrap();
    let out = exec.run_with_cache(&sink, &cancel, &mut cache).unwrap();
    std::fs::remove_dir_all(&dir).ok();

    match out.get("f").and_then(|m| m.get("text")) {
        Some(PortValue::Text(t)) => assert_eq!(t, "second", "file re-read on every run"),
        other => panic!("unexpected output: {other:?}"),
    }
}

#[test]
fn missing_input_names_the_port_label() {
    let reg = default_registry();
    let graph = SerializedGraph {
        nodes: vec![node("b", "hex_decode", json!({}))],
        edges: vec![],
    };
    let sink = Recorder::default();
    GraphExecutor::new(&reg, &graph)
        .unwrap()
        .run(&sink, &CancellationToken::new())
        .unwrap();
    let error = sink
        .take()
        .into_iter()
        .find_map(|e| match e {
            ProgressEvent::NodeFailed { error, .. } => Some(error),
            _ => None,
        })
        .expect("node fails without input");
    assert!(error.contains("缺少输入「"), "got: {error}");
}
