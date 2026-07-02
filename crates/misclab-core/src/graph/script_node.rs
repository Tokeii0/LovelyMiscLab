//! User-defined **script/program nodes**: a node whose execution body is an
//! external process (a Python script, a compiled exe, …). At run time we spawn
//! the process, feed inputs via stdin / argv / temp files, stream its stdout and
//! stderr live into the node log, and read outputs back from stdout (optionally
//! parsed as a JSON object for multiple named outputs) or from temp files.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::graph::port::{PortType, PortValue};
use crate::node::descriptor::{Cost, NodeDescriptor, ParamSpec, PortSpec};
use crate::node::registry::NodeFactory;
use crate::node::{Node, NodeCtx, NodeEnv, PortMap};
use crate::progress::LogLevel;

/// How a node input reaches the external process.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputDelivery {
    /// Written to the process's standard input (at most one such port).
    Stdin,
    /// Substituted (stringified) into the args template as a `{name}` token.
    Arg,
    /// Written to a temp file; the file path is substituted for `{name}`.
    File,
}

/// Where a node output is read from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputDelivery {
    /// Parsed from the process's stdout as `obj[name]` of a single JSON object.
    StdoutJson,
    /// Read from a temp file whose path was substituted for `{name}`.
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInputPort {
    pub name: String,
    pub label: String,
    pub port_type: PortType,
    pub delivery: InputDelivery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptOutputPort {
    pub name: String,
    pub label: String,
    pub port_type: PortType,
    pub delivery: OutputDelivery,
}

/// A user-defined external-process node. Persisted verbatim as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptModule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub description: String,
    /// Executable path/name, or `$tool:<key>` to resolve via `NodeEnv.tools` at run time.
    pub command: String,
    /// Argument template; `{name}` tokens are replaced by input/param values (quote-aware split).
    #[serde(default)]
    pub args_template: String,
    #[serde(default)]
    pub working_dir: Option<String>,
    /// 0 = unlimited.
    #[serde(default)]
    pub timeout_secs: u32,
    #[serde(default)]
    pub inputs: Vec<ScriptInputPort>,
    #[serde(default)]
    pub params: Vec<ParamSpec>,
    #[serde(default)]
    pub outputs: Vec<ScriptOutputPort>,
}

impl ScriptModule {
    pub fn descriptor(&self) -> NodeDescriptor {
        NodeDescriptor {
            id: self.id.clone(),
            category: if self.category.is_empty() { "自定义".into() } else { self.category.clone() },
            display_name: self.name.clone(),
            description: self.description.clone(),
            color: if self.color.is_empty() { "#8b5cf6".into() } else { self.color.clone() },
            inputs: self.inputs.iter().map(|p| PortSpec::new(&p.name, &p.label, p.port_type, false)).collect(),
            outputs: self.outputs.iter().map(|p| PortSpec::new(&p.name, &p.label, p.port_type, false)).collect(),
            params: self.params.clone(),
            cost: Cost::Heavy,
        }
    }

    pub fn factory(&self) -> NodeFactory {
        let module = self.clone();
        Arc::new(move || Arc::new(ScriptNode { module: module.clone() }))
    }
}

pub struct ScriptNode {
    module: ScriptModule,
}

// ---- pure helpers (unit-testable without spawning a process) ----

/// Keep temp filenames safe.
fn sanitize(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect()
}

fn truncate(s: &str, n: usize) -> String {
    let t: String = s.chars().take(n).collect();
    if t.chars().count() < s.chars().count() { format!("{t}…") } else { t }
}

/// Split an args template into tokens, honoring `"double quoted"` spans (so paths
/// with spaces survive as a single argument). Quotes are stripped.
pub fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut started = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_quote = !in_quote;
                started = true;
            }
            c if c.is_whitespace() && !in_quote => {
                if started {
                    out.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(cur);
    }
    out
}

/// Replace `{name}` placeholders within a single token. Unknown names are left as-is.
fn substitute(token: &str, subst: &HashMap<String, String>) -> String {
    let b = token.as_bytes();
    let mut out = String::with_capacity(token.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            if let Some(close) = token[i + 1..].find('}') {
                let name = &token[i + 1..i + 1 + close];
                if let Some(val) = subst.get(name) {
                    out.push_str(val);
                    i = i + 1 + close + 1;
                    continue;
                }
            }
            out.push('{');
            i += 1;
        } else {
            let ch = token[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn port_value_to_string(v: &PortValue) -> String {
    match v {
        PortValue::Text(s) => s.clone(),
        PortValue::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                (*n as i64).to_string()
            } else {
                n.to_string()
            }
        }
        PortValue::Bool(b) => b.to_string(),
        PortValue::StringList(v) => v.join("\n"),
        PortValue::Json(j) => j.to_string(),
        PortValue::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
        _ => String::new(),
    }
}

fn port_value_to_bytes(v: &PortValue) -> Vec<u8> {
    match v {
        PortValue::Bytes(b) => b.to_vec(),
        other => port_value_to_string(other).into_bytes(),
    }
}

fn json_to_arg(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Coerce a JSON value into a declared port type.
fn value_from_json(v: &serde_json::Value, ty: PortType) -> PortValue {
    use serde_json::Value as J;
    match ty {
        PortType::Text => PortValue::Text(match v {
            J::String(s) => s.clone(),
            other => other.to_string(),
        }),
        PortType::Number => match v {
            J::Number(n) => PortValue::Number(n.as_f64().unwrap_or(0.0)),
            J::String(s) => s.trim().parse::<f64>().map(PortValue::Number).unwrap_or_else(|_| PortValue::Text(s.clone())),
            _ => PortValue::Json(v.clone()),
        },
        PortType::Bool => match v {
            J::Bool(b) => PortValue::Bool(*b),
            _ => PortValue::Json(v.clone()),
        },
        PortType::StringList => match v {
            J::Array(a) => PortValue::StringList(
                a.iter().map(|x| match x { J::String(s) => s.clone(), o => o.to_string() }).collect(),
            ),
            _ => PortValue::Json(v.clone()),
        },
        PortType::Bytes => match v {
            J::String(s) => base64::engine::general_purpose::STANDARD
                .decode(s.trim())
                .map(|b| PortValue::Bytes(Arc::from(b.into_boxed_slice())))
                .unwrap_or_else(|_| PortValue::Text(s.clone())),
            _ => PortValue::Json(v.clone()),
        },
        _ => PortValue::Json(v.clone()),
    }
}

/// Coerce raw bytes (stdout fallback or a temp output file) into a declared port type.
fn value_from_raw(raw: &[u8], ty: PortType) -> PortValue {
    match ty {
        PortType::Bytes => PortValue::Bytes(Arc::from(raw.to_vec().into_boxed_slice())),
        PortType::Number => {
            let s = String::from_utf8_lossy(raw);
            s.trim().parse::<f64>().map(PortValue::Number).unwrap_or_else(|_| PortValue::Text(s.into_owned()))
        }
        PortType::Bool => {
            let s = String::from_utf8_lossy(raw);
            PortValue::Bool(matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on" | "是"))
        }
        PortType::StringList => {
            let s = String::from_utf8_lossy(raw);
            PortValue::StringList(s.lines().map(|l| l.to_string()).collect())
        }
        _ => PortValue::Text(String::from_utf8_lossy(raw).trim_end_matches(['\n', '\r']).to_string()),
    }
}

/// Build the output PortMap from the process's stdout bytes and the temp files it
/// wrote. Pure (no process spawning) so it can be unit-tested directly.
pub fn build_outputs(stdout: &[u8], outputs: &[ScriptOutputPort], scratch: &Path) -> Result<PortMap, CoreError> {
    let mut map = PortMap::new();

    // File-delivery outputs.
    for p in outputs.iter().filter(|p| matches!(p.delivery, OutputDelivery::File)) {
        let path = scratch.join(format!("out_{}", sanitize(&p.name)));
        let bytes = std::fs::read(&path)
            .map_err(|_| CoreError::Other(format!("输出「{}」的文件未生成：{}", p.label, path.display())))?;
        map.insert(p.name.clone(), value_from_raw(&bytes, p.port_type));
    }

    let stdout_ports: Vec<&ScriptOutputPort> =
        outputs.iter().filter(|p| matches!(p.delivery, OutputDelivery::StdoutJson)).collect();
    if stdout_ports.is_empty() {
        return Ok(map);
    }

    // A single stdout port may fall back to raw stdout when it isn't JSON.
    let single = outputs.len() == 1 && stdout_ports.len() == 1;
    let parsed = serde_json::from_slice::<serde_json::Value>(stdout).ok().filter(|v| v.is_object());
    match parsed {
        Some(obj) => {
            for p in &stdout_ports {
                if let Some(v) = obj.get(&p.name) {
                    map.insert(p.name.clone(), value_from_json(v, p.port_type));
                } else if single {
                    map.insert(p.name.clone(), value_from_raw(stdout, p.port_type));
                }
            }
        }
        None => {
            if single {
                let p = stdout_ports[0];
                map.insert(p.name.clone(), value_from_raw(stdout, p.port_type));
            } else {
                return Err(CoreError::Other(format!(
                    "脚本 stdout 不是合法 JSON 对象，无法映射多个输出端口。\nstdout: {}",
                    truncate(&String::from_utf8_lossy(stdout), 500)
                )));
            }
        }
    }
    Ok(map)
}

fn resolve_command(cmd: &str, env: &NodeEnv) -> Result<String, CoreError> {
    if let Some(key) = cmd.strip_prefix("$tool:") {
        env.tools
            .get(key)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .ok_or_else(|| CoreError::Other(format!("未配置工具「{key}」，请在设置中填写其可执行文件路径")))
    } else {
        Ok(cmd.to_string())
    }
}

/// Read a pipe line-by-line: stream each line to the node log and accumulate the
/// raw bytes (preserving exact output, including binary, for the caller).
fn drain(reader: impl Read, ctx: &NodeCtx, is_err: bool) -> Vec<u8> {
    let mut br = BufReader::new(reader);
    let mut acc: Vec<u8> = Vec::new();
    let mut line: Vec<u8> = Vec::new();
    loop {
        line.clear();
        match br.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                acc.extend_from_slice(&line);
                let text = String::from_utf8_lossy(&line);
                let trimmed = text.trim_end_matches(['\n', '\r']);
                if !trimmed.is_empty() {
                    let level = if is_err { LogLevel::Warn } else { LogLevel::Info };
                    let msg = if is_err { format!("[stderr] {trimmed}") } else { trimmed.to_string() };
                    ctx.log(level, msg);
                }
            }
        }
    }
    acc
}

enum End {
    Ok(ExitStatus),
    Cancelled,
    Timeout,
    Err(String),
}

impl Node for ScriptNode {
    fn run(&self, inputs: &PortMap, params: &serde_json::Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        let m = &self.module;
        let command = resolve_command(&m.command, ctx.env)?;
        let stdin_ports: Vec<&ScriptInputPort> =
            m.inputs.iter().filter(|p| matches!(p.delivery, InputDelivery::Stdin)).collect();
        if stdin_ports.len() > 1 {
            return Err(CoreError::Other("一个脚本节点最多只能有一个 stdin 输入端口".into()));
        }
        let scratch = std::env::temp_dir().join(format!("misclab_script_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&scratch).map_err(|e| CoreError::Other(format!("创建临时目录失败: {e}")))?;

        let result = self.run_inner(&command, inputs, params, ctx, &scratch, stdin_ports.first().copied());
        let _ = std::fs::remove_dir_all(&scratch);
        result
    }
}

impl ScriptNode {
    fn run_inner(
        &self,
        command: &str,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &NodeCtx,
        scratch: &Path,
        stdin_port: Option<&ScriptInputPort>,
    ) -> Result<PortMap, CoreError> {
        let m = &self.module;

        // Build the {name} substitution table and write File-delivery inputs.
        let mut subst: HashMap<String, String> = HashMap::new();
        for p in &m.inputs {
            match p.delivery {
                InputDelivery::File => {
                    let bytes = inputs.get(&p.name).map(port_value_to_bytes).unwrap_or_default();
                    let path = scratch.join(format!("in_{}", sanitize(&p.name)));
                    std::fs::write(&path, &bytes).map_err(|e| CoreError::Other(format!("写入输入文件失败: {e}")))?;
                    subst.insert(p.name.clone(), path.to_string_lossy().into_owned());
                }
                InputDelivery::Arg => {
                    let s = inputs.get(&p.name).map(port_value_to_string).unwrap_or_default();
                    subst.insert(p.name.clone(), s);
                }
                InputDelivery::Stdin => {}
            }
        }
        for p in m.outputs.iter().filter(|p| matches!(p.delivery, OutputDelivery::File)) {
            let path = scratch.join(format!("out_{}", sanitize(&p.name)));
            subst.insert(p.name.clone(), path.to_string_lossy().into_owned());
        }
        let obj = params.as_object();
        for spec in &m.params {
            let v = obj.and_then(|o| o.get(&spec.name)).unwrap_or(&spec.default);
            subst.insert(spec.name.clone(), json_to_arg(v));
        }

        let args: Vec<String> = tokenize(&m.args_template).iter().map(|t| substitute(t, &subst)).collect();

        let cwd = m
            .working_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| scratch.to_path_buf());

        let mut child = Command::new(command)
            .args(&args)
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CoreError::Other(format!("无法启动「{command}」: {e}（请检查路径/工具配置）")))?;

        let stdin_data: Option<Vec<u8>> =
            stdin_port.map(|p| inputs.get(&p.name).map(port_value_to_bytes).unwrap_or_default());
        let mut child_stdin = child.stdin.take();
        let child_stdout = child.stdout.take().expect("stdout piped");
        let child_stderr = child.stderr.take().expect("stderr piped");
        let timeout = m.timeout_secs;

        let (stdout_bytes, stderr_bytes, end) = std::thread::scope(|s| {
            if let (Some(mut si), Some(data)) = (child_stdin.take(), stdin_data) {
                s.spawn(move || {
                    let _ = si.write_all(&data); // dropping `si` closes stdin (EOF)
                });
            }
            let out_handle = s.spawn(|| drain(child_stdout, ctx, false));
            let err_handle = s.spawn(|| drain(child_stderr, ctx, true));

            let start = Instant::now();
            let end = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break End::Ok(status),
                    Ok(None) => {
                        if ctx.cancel.is_cancelled() {
                            let _ = child.kill();
                            break End::Cancelled;
                        }
                        if timeout > 0 && start.elapsed() > Duration::from_secs(timeout as u64) {
                            let _ = child.kill();
                            break End::Timeout;
                        }
                        std::thread::sleep(Duration::from_millis(120));
                    }
                    Err(e) => {
                        let _ = child.kill();
                        break End::Err(e.to_string());
                    }
                }
            };
            let out = out_handle.join().unwrap_or_default();
            let err = err_handle.join().unwrap_or_default();
            (out, err, end)
        });

        let stderr = String::from_utf8_lossy(&stderr_bytes);
        match end {
            End::Cancelled => return Err(CoreError::Cancelled),
            End::Timeout => return Err(CoreError::Other(format!("执行超时（>{timeout}s），已终止进程"))),
            End::Err(e) => return Err(CoreError::Other(format!("进程执行出错: {e}"))),
            End::Ok(status) => {
                if !status.success() {
                    let code = status.code().map(|c| c.to_string()).unwrap_or_else(|| "已被信号终止".into());
                    return Err(CoreError::Other(format!(
                        "脚本退出码 {code}。\nstderr: {}",
                        truncate(stderr.trim(), 800)
                    )));
                }
            }
        }

        build_outputs(&stdout_bytes, &m.outputs, scratch)
    }
}
