import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { ParamWidget, PortValue, ProgressMsg, SerializedGraph } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useProjectStore } from "@/store/project";
import { useRunStore } from "@/store/run";

// Module-level run coordination (single in-flight run; latest state coalesced).
let currentJob: string | null = null;
let inFlight = false;
let pending = false;

function now() {
  return new Date().toLocaleTimeString();
}

/** Coerce a connected port value into the JS type a param widget expects (mirrors
 * the Rust executor), so single-node execution honors param connections too. */
function coerceParam(v: PortValue, widget: ParamWidget): unknown {
  if (widget.kind === "number" || widget.kind === "slider") {
    if (v.type === "number") return v.value;
    if (v.type === "bool") return v.value ? 1 : 0;
    if (v.type === "text") {
      const n = parseFloat(v.value);
      return Number.isNaN(n) ? 0 : n;
    }
    return 0;
  }
  if (widget.kind === "toggle") {
    if (v.type === "bool") return v.value;
    if (v.type === "number") return v.value !== 0;
    if (v.type === "text")
      return ["true", "1", "yes", "on", "是"].includes(v.value.trim().toLowerCase());
    return false;
  }
  if (v.type === "text") return v.value;
  if (v.type === "number") return String(v.value);
  if (v.type === "bool") return String(v.value);
  if (v.type === "stringList") return v.value.join("\n");
  return "";
}

/** Serialize the current graph, excluding disabled nodes (and their edges). */
export function buildGraph(onlyIds?: Set<string>): SerializedGraph {
  const g = useGraphStore.getState();
  const enabled = g.nodes.filter((n) => !n.data.disabled && (!onlyIds || onlyIds.has(n.id)));
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
  useRunStore.getState().recordProgress(m);
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
    case "jobFailed":
      useRunStore.getState().setLastError(m.error);
      break;
    case "log":
      if (m.node) s.appendLog(m.node, { time: now(), level: m.level, message: m.message });
      break;
    default:
      break;
  }
}

function historyNodes() {
  const byId = useDescriptorStore.getState().byId;
  return useGraphStore.getState().nodes.map((n) => ({
    id: n.id,
    label: n.data.label || byId[n.data.descriptorId]?.displayName || n.data.descriptorId,
    descriptorId: n.data.descriptorId,
    status: n.data.status,
    error: n.data.error,
  }));
}

function startHistory(scope: "graph" | "node" | "debug", title: string, graph: SerializedGraph) {
  return useRunStore.getState().startHistory({
    scope,
    title,
    projectName: useProjectStore.getState().name,
    nodeCount: graph.nodes.length,
    edgeCount: graph.edges.length,
    nodes: historyNodes(),
  });
}

function finishHistory(entryId: string, elapsed: number, graph: SerializedGraph, error?: unknown) {
  const text = error ? String(error) : "";
  const ids = new Set(graph.nodes.map((n) => n.id));
  const status = text
    ? text.toLowerCase().includes("cancel")
      ? "cancelled"
      : "error"
    : useGraphStore.getState().nodes.some((n) => ids.has(n.id) && n.data.status === "error")
      ? "error"
      : "success";
  useRunStore.getState().finishHistory(entryId, {
    elapsed,
    status,
    error: text || undefined,
    nodes: historyNodes(),
  });
}

async function runSerializedGraph(graph: SerializedGraph, scope: "graph" | "debug", title: string) {
  if (!inTauri) return; // graphs execute in the Rust backend; no-op in a browser
  if (graph.nodes.length === 0) return;
  if (inFlight) {
    pending = true;
    return;
  }
  inFlight = true;
  useRunStore.getState().setRunning(true);
  useRunStore.getState().setLastError(null);
  const t0 = Date.now();
  const entryId = startHistory(scope, title, graph);
  let failure: unknown;
  try {
    const outputs = await api.runGraph(graph, handleEvent);
    const s = useGraphStore.getState();
    for (const [nodeId, portmap] of Object.entries(outputs)) {
      s.updateRuntime(nodeId, { outputs: portmap });
    }
  } catch (e) {
    failure = e;
    useRunStore.getState().setLastError(String(e));
    console.error("run_graph failed", e);
  } finally {
    inFlight = false;
    currentJob = null;
    const elapsed = Date.now() - t0;
    useRunStore.getState().setRunning(false);
    useRunStore.getState().setElapsed(elapsed);
    finishHistory(entryId, elapsed, graph, failure);
    if (pending) {
      pending = false;
      void executeGraph();
    }
  }
}

/** Run the whole graph. Backend caching makes this incremental. */
export async function executeGraph() {
  return runSerializedGraph(buildGraph(), "graph", "整图运行");
}

function upstreamNodeIds(nodeId: string): Set<string> {
  const g = useGraphStore.getState();
  const ids = new Set<string>();
  const visit = (id: string) => {
    if (ids.has(id)) return;
    ids.add(id);
    for (const edge of g.edges) {
      if (edge.target === id) visit(edge.source);
    }
  };
  visit(nodeId);
  return ids;
}

/** Run the selected node and all of its upstream dependencies as a debug subgraph. */
export async function executeToNode(nodeId: string) {
  const graph = buildGraph(upstreamNodeIds(nodeId));
  const node = useGraphStore.getState().nodes.find((n) => n.id === nodeId);
  await runSerializedGraph(graph, "debug", `运行到：${node?.data.label || nodeId}`);
}

/** Run a single node, gathering its inputs from upstream nodes' last outputs. */
export async function runSingleNode(nodeId: string) {
  const g = useGraphStore.getState();
  const node = g.nodes.find((n) => n.id === nodeId);
  if (!node) return;
  const descriptor = useDescriptorStore.getState().byId[node.data.descriptorId];

  const inputPortNames = new Set((descriptor?.inputs ?? []).map((p) => p.name));
  const inputs: Record<string, PortValue> = {};
  const paramOverrides: Record<string, unknown> = {};
  for (const e of g.edges) {
    if (e.target === nodeId && e.sourceHandle && e.targetHandle) {
      const src = g.nodes.find((n) => n.id === e.source);
      const val = src?.data.outputs?.[e.sourceHandle];
      if (!val) continue;
      if (inputPortNames.has(e.targetHandle)) {
        inputs[e.targetHandle] = val;
      } else {
        // An edge into a promoted parameter overrides that param's value.
        const spec = descriptor?.params.find((p) => p.name === e.targetHandle);
        if (spec) paramOverrides[e.targetHandle] = coerceParam(val, spec.widget);
      }
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
  const descriptorName = descriptor?.displayName ?? node.data.descriptorId;
  const entryId = useRunStore.getState().startHistory({
    scope: "node",
    title: `单节点运行：${node.data.label || descriptorName}`,
    projectName: useProjectStore.getState().name,
    nodeCount: 1,
    edgeCount: 0,
    nodes: historyNodes().filter((n) => n.id === nodeId),
  });
  const t0 = Date.now();
  try {
    const params = { ...node.data.params, ...paramOverrides };
    const outputs = await api.runNode(node.data.descriptorId, inputs, params);
    g.updateRuntime(nodeId, { status: "done", progress: 1, outputs });
    g.appendLog(nodeId, { time: now(), level: "success", message: "执行成功" });
    useRunStore.getState().appendHistoryEvent(entryId, {
      time: now(),
      level: "success",
      node: nodeId,
      message: "单节点执行成功",
    });
    useRunStore.getState().finishHistory(entryId, {
      status: "success",
      elapsed: Date.now() - t0,
      nodes: historyNodes().filter((n) => n.id === nodeId),
    });
  } catch (e) {
    g.updateRuntime(nodeId, { status: "error", error: String(e) });
    g.appendLog(nodeId, { time: now(), level: "error", message: String(e) });
    useRunStore.getState().appendHistoryEvent(entryId, {
      time: now(),
      level: "error",
      node: nodeId,
      message: String(e),
    });
    useRunStore.getState().finishHistory(entryId, {
      status: "error",
      elapsed: Date.now() - t0,
      error: String(e),
      nodes: historyNodes().filter((n) => n.id === nodeId),
    });
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
