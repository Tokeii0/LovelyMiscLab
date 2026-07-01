//! Node-graph commands: list descriptors (drives the palette), run a single node
//! standalone, and run a whole graph with streamed per-node progress.

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use misclab_core::cancel::CancellationToken;
use misclab_core::graph::executor::{GraphExecutor, GraphOutputs};
use misclab_core::graph::model::SerializedGraph;
use misclab_core::node::descriptor::NodeDescriptor;
use misclab_core::node::PortMap;
use misclab_core::progress::{LogLevel, NullSink, ProgressEvent, ProgressSink};

use crate::error::AppError;
use crate::state::AppState;

/// Progress messages streamed to the frontend over a Channel, keyed by node id.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProgressMsg {
    JobStarted { job: String },
    NodeEntered { node: String },
    NodeProgress { node: String, pct: f32 },
    NodeDone { node: String },
    NodeFailed { node: String, error: String },
    Log {
        node: Option<String>,
        level: String,
        message: String,
    },
    JobDone { job: String },
    JobFailed { job: String, error: String },
}

fn level_str(level: LogLevel) -> String {
    match level {
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
    }
    .to_string()
}

fn map_event(event: ProgressEvent) -> ProgressMsg {
    match event {
        ProgressEvent::NodeEntered { node } => ProgressMsg::NodeEntered { node },
        ProgressEvent::NodeProgress { node, pct } => ProgressMsg::NodeProgress { node, pct },
        ProgressEvent::NodeDone { node } => ProgressMsg::NodeDone { node },
        ProgressEvent::NodeFailed { node, error } => ProgressMsg::NodeFailed { node, error },
        ProgressEvent::Log {
            node,
            level,
            message,
        } => ProgressMsg::Log {
            node,
            level: level_str(level),
            message,
        },
    }
}

/// Bridges core `ProgressEvent`s onto a Tauri Channel.
struct ChannelSink {
    channel: Channel<ProgressMsg>,
}

impl ProgressSink for ChannelSink {
    fn emit(&self, event: ProgressEvent) {
        let _ = self.channel.send(map_event(event));
    }
}

/// All node descriptors, for the palette and generic node rendering.
#[tauri::command]
pub fn list_node_descriptors(state: State<'_, AppState>) -> Vec<NodeDescriptor> {
    state.registry.descriptors()
}

/// Run a single node standalone (the "quick tool" path).
#[tauri::command]
pub async fn run_node(
    state: State<'_, AppState>,
    descriptor_id: String,
    inputs: PortMap,
    params: serde_json::Value,
) -> Result<PortMap, AppError> {
    let registry = state.registry.clone();
    let env = state.settings.lock().expect("settings mutex poisoned").clone();
    let cancel = CancellationToken::new();
    let out = tauri::async_runtime::spawn_blocking(move || {
        GraphExecutor::run_node_with_env(
            registry.as_ref(),
            &descriptor_id,
            &inputs,
            &params,
            &env,
            &NullSink,
            &cancel,
        )
    })
    .await
    .map_err(|e| AppError::new("join", e.to_string()))??;
    Ok(out)
}

/// Run a whole graph, streaming per-node progress and returning all node outputs.
#[tauri::command]
pub async fn run_graph(
    state: State<'_, AppState>,
    graph: SerializedGraph,
    on_event: Channel<ProgressMsg>,
) -> Result<GraphOutputs, AppError> {
    let registry = state.registry.clone();
    let cache = state.cache.clone();
    let env = state.settings.lock().expect("settings mutex poisoned").clone();
    let cancel = CancellationToken::new();
    let job = state.jobs.start(cancel.clone());
    let _ = on_event.send(ProgressMsg::JobStarted { job: job.clone() });

    let sink = ChannelSink {
        channel: on_event.clone(),
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        let exec = GraphExecutor::new(registry.as_ref(), &graph)?.with_env(env);
        let mut cache = cache.lock().expect("cache mutex poisoned");
        exec.run_with_cache(&sink, &cancel, &mut cache)
    })
    .await;

    state.jobs.finish(&job);

    match result {
        Ok(Ok(outputs)) => {
            let _ = on_event.send(ProgressMsg::JobDone { job });
            Ok(outputs)
        }
        Ok(Err(core_err)) => {
            let _ = on_event.send(ProgressMsg::JobFailed {
                job,
                error: core_err.to_string(),
            });
            Err(core_err.into())
        }
        Err(join_err) => {
            let _ = on_event.send(ProgressMsg::JobFailed {
                job,
                error: join_err.to_string(),
            });
            Err(AppError::new("join", join_err.to_string()))
        }
    }
}

/// Cancel a running graph job by id.
#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, job: String) {
    state.jobs.cancel(&job);
}

/// Clear the incremental-execution cache (used by the Stop control).
#[tauri::command]
pub fn reset_run(state: State<'_, AppState>) {
    if let Ok(mut cache) = state.cache.lock() {
        cache.clear();
    }
}
