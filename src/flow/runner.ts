import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { PortValue, ProgressMsg, SerializedGraph } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useRunStore } from "@/store/run";

// Module-level run coordination (single in-flight run; latest state coalesced).
let currentJob: string | null = null;
let inFlight = false;
let pending = false;

function now() {
  return new Date().toLocaleTimeString();
}

/** Serialize the current graph, excluding disabled nodes (and their edges). */
function buildGraph(): SerializedGraph {
  const g = useGraphStore.getState();
  const enabled = g.nodes.filter((n) => !n.data.disabled);
  const ids = new Set(enabled.map((n) => n.id));
  return {
    nodes: enabled.map((n) => ({
      id: n.id,
      descriptorId: n.data.descriptorId,
      params: n.data.params,
      position: [n.position.x, n.position.y],
    })),
    edges: g.edges
      .filter(
        (e) =>
          e.sourceHandle && e.targetHandle && ids.has(e.source) && ids.has(e.target)
      )
      .map((e) => ({
        from: { node: e.source, port: e.sourceHandle as string },
        to: { node: e.target, port: e.targetHandle as string },
      })),
  };
}

function handleEvent(m: ProgressMsg) {
  const s = useGraphStore.getState();
  switch (m.kind) {
    case "jobStarted":
      currentJob = m.job;
      break;
    case "nodeEntered":
      s.updateRuntime(m.node, {
        status: "running",
        progress: 0,
        error: undefined,
        logs: [{ time: now(), level: "info", message: "开始执行" }],
      });
      break;
    case "nodeProgress":
      s.updateRuntime(m.node, { progress: m.pct });
      break;
    case "nodeDone":
      s.updateRuntime(m.node, { status: "done", progress: 1 });
      s.appendLog(m.node, { time: now(), level: "success", message: "执行成功" });
      break;
    case "nodeFailed":
      s.updateRuntime(m.node, { status: "error", error: m.error });
      s.appendLog(m.node, { time: now(), level: "error", message: m.error });
      break;
    case "log":
      if (m.node) s.appendLog(m.node, { time: now(), level: m.level, message: m.message });
      break;
    default:
      break;
  }
}

/** Run the whole graph. Backend caching makes this incremental. */
export async function executeGraph() {
  if (!inTauri) return; // graphs execute in the Rust backend; no-op in a browser
  if (useGraphStore.getState().nodes.length === 0) return;
  if (inFlight) {
    pending = true;
    return;
  }
  inFlight = true;
  useRunStore.getState().setRunning(true);
  const t0 = Date.now();
  try {
    const outputs = await api.runGraph(buildGraph(), handleEvent);
    const s = useGraphStore.getState();
    for (const [nodeId, portmap] of Object.entries(outputs)) {
      s.updateRuntime(nodeId, { outputs: portmap });
    }
  } catch (e) {
    console.error("run_graph failed", e);
  } finally {
    inFlight = false;
    currentJob = null;
    useRunStore.getState().setRunning(false);
    useRunStore.getState().setElapsed(Date.now() - t0);
    if (pending) {
      pending = false;
      void executeGraph();
    }
  }
}

/** Run a single node, gathering its inputs from upstream nodes' last outputs. */
export async function runSingleNode(nodeId: string) {
  const g = useGraphStore.getState();
  const node = g.nodes.find((n) => n.id === nodeId);
  if (!node) return;
  const descriptor = useDescriptorStore.getState().byId[node.data.descriptorId];

  const inputs: Record<string, PortValue> = {};
  for (const e of g.edges) {
    if (e.target === nodeId && e.sourceHandle && e.targetHandle) {
      const src = g.nodes.find((n) => n.id === e.source);
      const val = src?.data.outputs?.[e.sourceHandle];
      if (val) inputs[e.targetHandle] = val;
    }
  }

  const missing = (descriptor?.inputs ?? []).filter(
    (p) => p.required && !(p.name in inputs)
  );
  if (missing.length > 0) {
    g.updateRuntime(nodeId, {
      status: "error",
      error: `缺少输入：${missing.map((p) => p.label).join("、")}（请先执行上游节点）`,
    });
    g.appendLog(nodeId, { time: now(), level: "error", message: "缺少输入" });
    return;
  }

  if (!inTauri) {
    g.updateRuntime(nodeId, { status: "error", error: "浏览器预览无法执行节点" });
    return;
  }

  g.updateRuntime(nodeId, {
    status: "running",
    progress: 0,
    error: undefined,
    logs: [{ time: now(), level: "info", message: "单独执行" }],
  });
  try {
    const outputs = await api.runNode(node.data.descriptorId, inputs, node.data.params);
    g.updateRuntime(nodeId, { status: "done", progress: 1, outputs });
    g.appendLog(nodeId, { time: now(), level: "success", message: "执行成功" });
  } catch (e) {
    g.updateRuntime(nodeId, { status: "error", error: String(e) });
    g.appendLog(nodeId, { time: now(), level: "error", message: String(e) });
  }
}

/** Pause live mode (halt the current run; completed nodes stay cached). */
export async function pauseRun() {
  useRunStore.getState().setMode("paused");
  if (currentJob) {
    try {
      await api.cancelJob(currentJob);
    } catch {
      /* ignore */
    }
  }
}

/** Stop: cancel, clear the incremental cache, and reset node runtime state. */
export async function stopRun() {
  useRunStore.getState().setMode("idle");
  if (currentJob) {
    try {
      await api.cancelJob(currentJob);
    } catch {
      /* ignore */
    }
  }
  try {
    await api.resetRun();
  } catch {
    /* ignore (unavailable outside Tauri) */
  }
  useGraphStore.getState().resetRuntime();
}
