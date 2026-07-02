//! End-to-end test: start the real embedded server and drive it over HTTP through
//! the MCP protocol (initialize → tools/list → tools/call), plus the bearer gate
//! and the canvas-emit path. Uses a mock [`AppBridge`] so no Tauri app is needed.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use misclab_core::node::NodeEnv;
use misclab_core::nodes::default_registry;

use crate::mcp::state::{AppBridge, CanvasSnapshot, McpState};

struct MockBridge {
    dir: PathBuf,
    emitted: Mutex<Vec<CanvasSnapshot>>,
}

impl AppBridge for MockBridge {
    fn emit_canvas(&self, snapshot: &CanvasSnapshot) {
        self.emitted.lock().unwrap().push(snapshot.clone());
    }
    fn app_data_dir(&self) -> Option<PathBuf> {
        Some(self.dir.clone())
    }
}

const TOKEN: &str = "itest-token";

/// POST a JSON-RPC body; return (raw response, session id from headers).
fn post(addr: &str, token: &str, session: Option<&str>, body: &str) -> (String, String) {
    let mut req = format!(
        "POST /mcp HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nAuthorization: Bearer {token}\r\n\
         Content-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(s) = session {
        req.push_str(&format!("mcp-session-id: {s}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);

    let mut stream = TcpStream::connect(addr).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.write_all(req.as_bytes()).unwrap();
    let mut resp = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => resp.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&resp).to_string();
    let sid = text
        .lines()
        .find_map(|l| l.to_lowercase().strip_prefix("mcp-session-id: ").map(|_| l["mcp-session-id: ".len()..].trim().to_string()))
        .unwrap_or_default();
    (text, sid)
}

#[test]
fn server_end_to_end_over_http() {
    let dir = std::env::temp_dir().join(format!("misclab-mcp-itest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let bridge = Arc::new(MockBridge { dir: dir.clone(), emitted: Mutex::new(Vec::new()) });

    let state = McpState {
        registry: Arc::new(default_registry()),
        composites: Arc::new(Mutex::new(Vec::new())),
        scripts: Arc::new(Mutex::new(Vec::new())),
        cache: Arc::new(Mutex::new(Default::default())),
        settings: Arc::new(Mutex::new(NodeEnv::default())),
        canvas: Arc::new(Mutex::new(CanvasSnapshot::default())),
        app: bridge.clone(),
        token: Arc::new(Some(TOKEN.to_string())),
    };

    let addr = "127.0.0.1:39222";
    let handle = crate::mcp::start(state, addr.parse().unwrap()).expect("server starts");

    // --- bearer gate: wrong token is rejected -------------------------------
    let (bad, _) = post(addr, "wrong", None, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
    assert!(bad.starts_with("HTTP/1.1 401"), "wrong token should 401, got: {}", bad.lines().next().unwrap_or(""));

    // --- initialize ---------------------------------------------------------
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"itest","version":"0"}}}"#;
    let (resp, sid) = post(addr, TOKEN, None, init);
    assert!(resp.contains("\"result\""), "initialize failed: {resp}");
    assert!(!sid.is_empty(), "no session id returned");
    // The initialized notification (courtesy).
    let _ = post(addr, TOKEN, Some(&sid), r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);

    // --- tools/list contains our tools --------------------------------------
    let (tools, _) = post(addr, TOKEN, Some(&sid), r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    for expected in ["list_nodes", "run_node", "run_graph", "add_node", "connect", "get_canvas"] {
        assert!(tools.contains(expected), "tools/list missing {expected}: {tools}");
    }

    // --- tools/call list_nodes (query) --------------------------------------
    let list = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_nodes","arguments":{"query":"base64"}}}"#;
    let (list_resp, _) = post(addr, TOKEN, Some(&sid), list);
    assert!(list_resp.contains("base64"), "list_nodes(base64) empty: {list_resp}");

    // --- tools/call run_node (hash "hello" -> md5) --------------------------
    let run = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"run_node","arguments":{"descriptorId":"hash","inputs":{"data":"hello"},"params":{"algorithm":"MD5"}}}}"#;
    let (run_resp, _) = post(addr, TOKEN, Some(&sid), run);
    assert!(
        run_resp.contains("5d41402abc4b2a76b9719d911017c592"),
        "run_node hash(MD5,\"hello\") wrong: {run_resp}"
    );

    // --- tools/call add_node emits a canvas update --------------------------
    let add = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"add_node","arguments":{"descriptorId":"text_input"}}}"#;
    let (add_resp, _) = post(addr, TOKEN, Some(&sid), add);
    assert!(add_resp.contains("ai_text_input_1"), "add_node id wrong: {add_resp}");
    let emitted = bridge.emitted.lock().unwrap();
    assert_eq!(emitted.len(), 1, "add_node should emit exactly one canvas update");
    assert_eq!(emitted[0].nodes.len(), 1);
    assert_eq!(emitted[0].rev, 1, "rev should advance to 1");

    handle.stop();
    std::fs::remove_dir_all(&dir).ok();
}
