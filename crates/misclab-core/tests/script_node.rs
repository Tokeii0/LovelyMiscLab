//! External script/program node: pure output-building + real-process integration.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use misclab_core::cancel::CancellationToken;
use misclab_core::error::CoreError;
use misclab_core::graph::executor::GraphExecutor;
use misclab_core::graph::port::{PortType, PortValue};
use misclab_core::graph::script_node::{
    build_outputs, tokenize, InputDelivery, OutputDelivery, ScriptInputPort, ScriptModule, ScriptOutputPort,
};
use misclab_core::node::descriptor::ParamSpec;
use misclab_core::node::PortMap;
use misclab_core::nodes::default_registry;
use misclab_core::progress::NullSink;
use serde_json::json;

fn out(name: &str, ty: PortType, d: OutputDelivery) -> ScriptOutputPort {
    ScriptOutputPort { name: name.into(), label: name.into(), port_type: ty, delivery: d }
}
fn inp(name: &str, ty: PortType, d: InputDelivery) -> ScriptInputPort {
    ScriptInputPort { name: name.into(), label: name.into(), port_type: ty, delivery: d }
}
fn script(id: &str, command: &str, args: &str) -> ScriptModule {
    ScriptModule {
        id: id.into(),
        name: id.into(),
        category: String::new(),
        color: String::new(),
        description: String::new(),
        command: command.into(),
        args_template: args.into(),
        working_dir: None,
        timeout_secs: 30,
        inputs: vec![],
        params: vec![],
        outputs: vec![],
    }
}
fn text_of(m: &PortMap, port: &str) -> String {
    match m.get(port) {
        Some(PortValue::Text(s)) => s.clone(),
        o => panic!("expected Text at '{port}', got {o:?}"),
    }
}
fn run_script(m: &ScriptModule, inputs: HashMap<String, PortValue>, params: serde_json::Value) -> Result<PortMap, CoreError> {
    let mut reg = default_registry();
    reg.register(m.descriptor(), m.factory());
    GraphExecutor::run_node(&reg, &m.id, &inputs, &params, &NullSink, &CancellationToken::new())
}

// ---- pure (no process) ----

#[test]
fn build_outputs_multi_json() {
    let outs = vec![
        out("text", PortType::Text, OutputDelivery::StdoutJson),
        out("count", PortType::Number, OutputDelivery::StdoutJson),
    ];
    let m = build_outputs(br#"{"text":"hi","count":5}"#, &outs, Path::new(".")).unwrap();
    assert_eq!(text_of(&m, "text"), "hi");
    assert!(matches!(m.get("count"), Some(PortValue::Number(n)) if *n == 5.0));
}

#[test]
fn build_outputs_single_raw_fallback() {
    let outs = vec![out("out", PortType::Text, OutputDelivery::StdoutJson)];
    let m = build_outputs(b"not json\n", &outs, Path::new(".")).unwrap();
    assert_eq!(text_of(&m, "out"), "not json");
}

#[test]
fn build_outputs_bytes_base64() {
    let outs = vec![out("b", PortType::Bytes, OutputDelivery::StdoutJson)];
    let m = build_outputs(br#"{"b":"aGk="}"#, &outs, Path::new(".")).unwrap();
    assert!(matches!(m.get("b"), Some(PortValue::Bytes(b)) if &b[..] == b"hi"));
}

#[test]
fn build_outputs_multi_non_json_errors() {
    let outs = vec![
        out("a", PortType::Text, OutputDelivery::StdoutJson),
        out("b", PortType::Text, OutputDelivery::StdoutJson),
    ];
    assert!(build_outputs(b"nope", &outs, Path::new(".")).is_err());
}

#[test]
fn tokenize_quotes() {
    assert_eq!(tokenize(r#""a b" c {d}"#), vec!["a b", "c", "{d}"]);
    assert_eq!(tokenize("/C echo {msg}"), vec!["/C", "echo", "{msg}"]);
    assert_eq!(tokenize("   "), Vec::<String>::new());
}

// ---- real process (Windows built-in commands; no interpreters required) ----

#[cfg(windows)]
#[test]
fn proc_echo_arg() {
    let mut m = script("t_echo", "cmd", "/C echo {msg}");
    m.params = vec![ParamSpec::text("msg", "msg", "", false)];
    m.outputs = vec![out("out", PortType::Text, OutputDelivery::StdoutJson)];
    let r = run_script(&m, HashMap::new(), json!({ "msg": "hello" })).unwrap();
    assert_eq!(text_of(&r, "out"), "hello");
}

#[cfg(windows)]
#[test]
fn proc_nonzero_exit_errors() {
    let mut m = script("t_exit", "cmd", "/C exit 3");
    m.outputs = vec![out("out", PortType::Text, OutputDelivery::StdoutJson)];
    assert!(run_script(&m, HashMap::new(), json!({})).is_err());
}

#[cfg(windows)]
#[test]
fn proc_file_roundtrip() {
    let mut m = script("t_copy", "cmd", "/C copy /Y {inf} {outf}");
    m.inputs = vec![inp("inf", PortType::Bytes, InputDelivery::File)];
    m.outputs = vec![out("outf", PortType::Bytes, OutputDelivery::File)];
    let mut inputs = HashMap::new();
    inputs.insert("inf".into(), PortValue::Bytes(Arc::from(b"ROUNDTRIP".to_vec().into_boxed_slice())));
    let r = run_script(&m, inputs, json!({})).unwrap();
    assert!(matches!(r.get("outf"), Some(PortValue::Bytes(b)) if &b[..] == b"ROUNDTRIP"));
}

#[cfg(windows)]
#[test]
fn proc_stdin_echo() {
    // `sort` with no args reads stdin, writes to stdout; single line = identity.
    let mut m = script("t_sort", "sort", "");
    m.inputs = vec![inp("data", PortType::Text, InputDelivery::Stdin)];
    m.outputs = vec![out("out", PortType::Text, OutputDelivery::StdoutJson)];
    let mut inputs = HashMap::new();
    inputs.insert("data".into(), PortValue::Text("STDIN_LINE".into()));
    let r = run_script(&m, inputs, json!({})).unwrap();
    assert_eq!(text_of(&r, "out"), "STDIN_LINE");
}

#[cfg(windows)]
#[test]
fn proc_timeout_kills() {
    // ping ~4s directly (not via cmd, so kill() reaches the real process).
    let mut m = script("t_timeout", "ping", "-n 5 127.0.0.1");
    m.timeout_secs = 1;
    m.outputs = vec![out("out", PortType::Text, OutputDelivery::StdoutJson)];
    let t0 = Instant::now();
    let r = run_script(&m, HashMap::new(), json!({}));
    assert!(r.is_err());
    assert!(t0.elapsed() < Duration::from_secs(4), "should be killed early, took {:?}", t0.elapsed());
}
