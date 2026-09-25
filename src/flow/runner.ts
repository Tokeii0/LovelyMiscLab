// Run coordination. Every run — the 运行 button, live mode, "运行到此节点",
// a node's ▶ — goes through one queue: only one run is in flight, the most
// recent request waits behind it, and Stop cancels without throwing results away.

import { api } from "@/lib/bindings";
import { inTauri, mockCancelJob, mockRunGraph } from "@/lib/devMocks";
import { errorMessage, isCancelled } from "@/lib/errors";
import type { ParamWidget, PortValue, ProgressMsg, SerializedGraph } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useProjectStore } from "@/store/project";
import { useRunStore, type ActiveRun } from "@/store/run";
import { toast } from "@/store/toast";

export type RunRequest =
  | { kind: "graph" }
  | { kind: "live" }
  | { kind: "toNode"; nodeId: string }
  | { kind: "node"; nodeId: string };

let currentJob: string | null = null;
let inFlight = false;
let queued: RunRequest | null = null;
/** The one run-history entry live mode keeps updating instead of adding hundreds. */
let liveEntryId: string | null = null;

const LIVE_HINT = "耗时节点：实时模式不会自动运行，点「运行」执行";

function now() {
  return new Date().toLocaleTimeString();
}

/** Coerce a connected port value into the JS type a param widget expects (mirrors
 * the Rust executor), so single-node execution honors param connections too.
 * `undefined` = no usable value (the param keeps its own setting). */
function coerceParam(v: PortValue, widget: ParamWidget): unknown {
  if (widget.kind === "number" || widget.kind === "slider") {
    if (v.type === "number") return v.value;
    if (v.type === "bool") return v.value ? 1 : 0;
    if (v.type === "text") {
      const n = parseFloat(v.value);
      return Number.isNaN(n) ? undefined : n;
    }
    return undefined;
  }
  if (widget.kind === "toggle") {
    if (v.type === "bool") return v.value;
    if (v.type === "number") return v.value !== 0;
    if (v.type === "text")
      return ["true", "1", "yes", "on", "是"].includes(v.value.trim().toLowerCase());
    return undefined;
  }
  if (v.type === "text") return v.value;
  if (v.type === "number") return String(v.value);
  if (v.type === "bool") return String(v.value);
  if (v.type === "stringList") return v.value.join("\n");
  return undefined;
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
      .filter((e) => e.sourceHandle && e.targetHandle && ids.has(e.source) && ids.has(e.target))
      .map((e) => ({
        from: { node: e.source, port: e.sourceHandle as string },
        to: { node: e.target, port: e.targetHandle as string },
      })),
  };
}

function upstreamNodeIds(nodeId: string): Set<string> {
  const g = useGraphStore.getState();
  const ids = new Set<string>();
  const visit = (id: string) => {
    if (ids.has(id)) return;
    ids.add(id);
    for (const edge of g.edges) if (edge.target === id) visit(edge.source);
  };
  visit(nodeId);
  return ids;
}

/** Heavy nodes (cracking, AI, HTTP, external programs) and everything downstream:
 * live mode leaves these for an explicit run. */
function heavyAndDownstream(): Set<string> {
  const g = useGraphStore.getState();
  const byId = useDescriptorStore.getState().byId;
  const out = new Set<string>();
  const stack = g.nodes.filter((n) => byId[n.data.descriptorId]?.cost === "heavy").map((n) => n.id);
  while (stack.length) {
    const id = stack.pop()!;
    if (out.has(id)) continue;
    out.add(id);
    for (const e of g.edges) if (e.source === id) stack.push(e.target);
  }
  return out;
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
        hint: undefined,
        logs: [{ time: now(), level: "info", message: "开始执行" }],
      });
      break;
    case "nodeProgress":
      s.updateRuntime(m.node, { progress: m.pct });
      break;
    case "nodeDone":
      s.updateRuntime(m.node, {
        status: "done",
        progress: 1,
        error: undefined,
        stale: false,
        hint: undefined,
        ...(m.outputs ? { outputs: m.outputs } : {}),
      });
      if (!m.cached) s.appendLog(m.node, { time: now(), level: "success", message: "执行成功" });
      break;
    case "nodeSkipped":
      s.updateRuntime(m.node, {
        status: "skipped",
        outputs: undefined,
        error: undefined,
        stale: false,
        hint: m.reason,
      });
      break;
    case "nodeFailed":
      // A failed node's previous outputs no longer describe anything real.
      s.updateRuntime(m.node, {
        status: "error",
        error: m.error,
        outputs: undefined,
        stale: false,
        hint: undefined,
      });
      s.appendLog(m.node, { time: now(), level: "error", message: m.error });
      break;
    case "jobFailed":
      if (!/取消|cancel/i.test(m.error)) useRunStore.getState().setLastError(m.error);
      break;
    case "log":
      if (m.node) s.appendLog(m.node, { time: now(), level: m.level, message: m.message });
      break;
    default:
      break;
  }
}

/** Nodes left "running" by a cancelled run go back to their previous look. */
function settleInterrupted() {
  const s = useGraphStore.getState();
  for (const n of s.nodes) {
    if (n.data.status === "running") {
      s.updateRuntime(n.id, {
        status: n.data.outputs ? "done" : "idle",
        progress: 0,
        stale: !!n.data.outputs,
      });
      s.appendLog(n.id, { time: now(), level: "warn", message: "已取消" });
    }
  }
}

/** Truncate an output value so run history stays small enough for localStorage. */
function sanitizeValue(v: PortValue): PortValue {
  const cap = (s: string, max: number) => (s.length > max ? `${s.slice(0, max)}…` : s);
  switch (v.type) {
    case "text":
      return { type: "text", value: cap(v.value, 20000) };
    case "bytes":
      return { type: "bytes", value: v.value.slice(0, 8192) };
    case "image":
      // Keep the data-URL unless it's very large (would bloat localStorage).
      return v.value.length > 300000
        ? { type: "text", value: "[图片较大，未在记录中保存预览]" }
        : v;
    case "stringList":
      return { type: "stringList", value: v.value.slice(0, 200).map((s) => cap(s, 2000)) };
    case "candidates":
      return { type: "candidates", value: v.value.slice(0, 50) };
    case "json":
    case "fingerprint": {
      const s = JSON.stringify(v.value);
      return s && s.length > 8000 ? { type: "text", value: cap(s, 8000) } : v;
    }
    default:
      return v;
  }
}

function sanitizeOutputs(
  outputs?: Record<string, PortValue>
): Record<string, PortValue> | undefined {
  if (!outputs) return undefined;
  const out: Record<string, PortValue> = {};
  for (const [k, v] of Object.entries(outputs)) out[k] = sanitizeValue(v);
  return Object.keys(out).length ? out : undefined;
}

function historyNodes(only?: Set<string>) {
  const byId = useDescriptorStore.getState().byId;
  return useGraphStore
    .getState()
    .nodes.filter((n) => !only || only.has(n.id))
    .map((n) => ({
      id: n.id,
      label: n.data.label || byId[n.data.descriptorId]?.displayName || n.data.descriptorId,
      descriptorId: n.data.descriptorId,
      status: n.data.status,
      error: n.data.error,
      outputs: sanitizeOutputs(n.data.outputs),
    }));
}

// ---------------------------------------------------------------------------
// The queue
// ---------------------------------------------------------------------------

/** Merge a new request with one already waiting: an explicit run beats a live
 * re-run; otherwise the newest request wins. */
function mergeQueued(prev: RunRequest | null, next: RunRequest): RunRequest {
  if (prev && prev.kind !== "live" && next.kind === "live") return prev;
  return next;
}

function describe(req: RunRequest): ActiveRun {
  const label = (id: string) =>
    useGraphStore.getState().nodes.find((n) => n.id === id)?.data.label || id;
  switch (req.kind) {
    case "graph":
      return { kind: "graph", title: "整图运行" };
    case "live":
      return { kind: "live", title: "实时运行" };
    case "toNode":
      return { kind: "toNode", title: `运行到：${label(req.nodeId)}` };
    case "node":
      return { kind: "node", title: `单节点运行：${label(req.nodeId)}` };
  }
}

export async function requestRun(req: RunRequest): Promise<void> {
  if (inFlight) {
    queued = mergeQueued(queued, req);
    return;
  }
  inFlight = true;
  const active = describe(req);
  useRunStore.getState().setRunning(true, active);
  try {
    if (req.kind === "node") await execNode(req.nodeId, active);
    else await execGraph(req, active);
  } finally {
    inFlight = false;
    currentJob = null;
    useRunStore.getState().setRunning(false);
    const next = queued;
    queued = null;
    // A live re-run only makes sense while live mode is still on.
    if (next && (next.kind !== "live" || useRunStore.getState().mode === "live")) {
      void requestRun(next);
    }
  }
}

async function execGraph(req: Exclude<RunRequest, { kind: "node" }>, active: ActiveRun) {
  let only: Set<string> | undefined;
  if (req.kind === "toNode") only = upstreamNodeIds(req.nodeId);
  if (req.kind === "live") {
    // Live mode skips expensive nodes; tell them why they didn't update.
    const heavy = heavyAndDownstream();
    if (heavy.size) {
      const g = useGraphStore.getState();
      for (const n of g.nodes) {
        if (
          heavy.has(n.id) &&
          (n.data.status === "idle" || n.data.stale) &&
          n.data.hint !== LIVE_HINT
        ) {
          g.updateRuntime(n.id, { hint: LIVE_HINT });
        }
      }
      only = new Set(g.nodes.filter((n) => !heavy.has(n.id)).map((n) => n.id));
    }
  }
  const graph = buildGraph(only);
  if (graph.nodes.length === 0) return;

  useRunStore.getState().setLastError(null);
  const t0 = Date.now();
  const ids = new Set(graph.nodes.map((n) => n.id));
  const entryId = useRunStore.getState().startHistory(
    {
      scope: req.kind === "toNode" ? "debug" : "graph",
      title: active.title,
      projectName: useProjectStore.getState().name,
      nodeCount: graph.nodes.length,
      edgeCount: graph.edges.length,
      nodes: historyNodes(ids),
    },
    req.kind === "live" ? liveEntryId : null
  );
  if (req.kind === "live") liveEntryId = entryId;

  let failure: unknown;
  try {
    if (inTauri) await api.runGraph(graph, handleEvent);
    else await mockRunGraph(graph, handleEvent);
  } catch (e) {
    failure = e;
    if (isCancelled(e)) settleInterrupted();
    else useRunStore.getState().setLastError(errorMessage(e));
  } finally {
    const elapsed = Date.now() - t0;
    useRunStore.getState().setElapsed(elapsed);
    const failedNode = useGraphStore
      .getState()
      .nodes.some((n) => ids.has(n.id) && n.data.status === "error");
    useRunStore.getState().finishHistory(entryId, {
      elapsed,
      status: failure
        ? isCancelled(failure)
          ? "cancelled"
          : "error"
        : failedNode
          ? "error"
          : "success",
      error: failure ? errorMessage(failure) : undefined,
      nodes: historyNodes(ids),
    });
  }
}

/** Run one node with the upstream results already on the canvas. When those are
 * missing or out of date (fresh graph, edits upstream) it runs the upstream too. */
async function execNode(nodeId: string, active: ActiveRun) {
  const g = useGraphStore.getState();
  const node = g.nodes.find((n) => n.id === nodeId);
  if (!node) return;
  if (node.data.disabled) {
    toast.info("该节点已禁用", { detail: "先在右键菜单或节点工具栏中启用它" });
    return;
  }
  const descriptor = useDescriptorStore.getState().byId[node.data.descriptorId];

  const inputPortNames = new Set((descriptor?.inputs ?? []).map((p) => p.name));
  const inputs: Record<string, PortValue> = {};
  const paramOverrides: Record<string, unknown> = {};
  let upstreamMissing = false;
  for (const e of g.edges) {
    if (e.target !== nodeId || !e.sourceHandle || !e.targetHandle) continue;
    const src = g.nodes.find((n) => n.id === e.source);
    const val = src?.data.outputs?.[e.sourceHandle];
    if (!val || src?.data.stale) {
      upstreamMissing = true;
      continue;
    }
    if (inputPortNames.has(e.targetHandle)) {
      inputs[e.targetHandle] = val;
    } else {
      // An edge into a promoted parameter overrides that param's value.
      const spec = descriptor?.params.find((p) => p.name === e.targetHandle);
      const coerced = spec ? coerceParam(val, spec.widget) : undefined;
      if (coerced !== undefined) paramOverrides[e.targetHandle] = coerced;
    }
  }
  if (upstreamMissing || !inTauri) {
    await execGraph({ kind: "toNode", nodeId }, active);
    return;
  }

  handleEvent({ kind: "nodeEntered", node: nodeId });
  const entryId = useRunStore.getState().startHistory({
    scope: "node",
    title: active.title,
    projectName: useProjectStore.getState().name,
    nodeCount: 1,
    edgeCount: 0,
    nodes: historyNodes(new Set([nodeId])),
  });
  const t0 = Date.now();
  try {
    const params = { ...node.data.params, ...paramOverrides };
    // Streams progress/logs so long nodes (e.g. bkcrack) drive the progress bar,
    // and registers a job so 停止 can cancel it.
    const outputs = await api.runNodeStreamed(
      node.data.descriptorId,
      nodeId,
      inputs,
      params,
      (m) => {
        if (m.kind === "jobStarted") currentJob = m.job;
        else if (m.kind === "nodeProgress" || m.kind === "log") handleEvent(m);
      }
    );
    handleEvent({ kind: "nodeDone", node: nodeId, outputs, cached: false });
    useRunStore.getState().finishHistory(entryId, {
      status: "success",
      elapsed: Date.now() - t0,
      nodes: historyNodes(new Set([nodeId])),
    });
  } catch (e) {
    const msg = errorMessage(e);
    if (isCancelled(e)) settleInterrupted();
    else handleEvent({ kind: "nodeFailed", node: nodeId, error: msg });
    useRunStore.getState().finishHistory(entryId, {
      status: isCancelled(e) ? "cancelled" : "error",
      elapsed: Date.now() - t0,
      error: msg,
      nodes: historyNodes(new Set([nodeId])),
    });
  }
}

// ---------------------------------------------------------------------------
// Public controls
// ---------------------------------------------------------------------------

/** 运行 / Ctrl+Enter: run the whole graph once, heavy nodes included. */
export function runGraph() {
  return requestRun({ kind: "graph" });
}

/** Run a node together with everything it depends on. */
export function runToNode(nodeId: string) {
  return requestRun({ kind: "toNode", nodeId });
}

/** Run just this node on its upstream's current results. */
export function runNode(nodeId: string) {
  return requestRun({ kind: "node", nodeId });
}

/** Turn live mode on/off. Turning it on runs right away. */
export function setLiveMode(on: boolean) {
  useRunStore.getState().setMode(on ? "live" : "manual");
  if (on) void requestRun({ kind: "live" });
  else if (queued?.kind === "live") queued = null;
}

/** 停止: cancel what's running and anything queued. Results so far stay. */
export async function stopRun() {
  queued = null;
  if (useRunStore.getState().mode === "live") useRunStore.getState().setMode("manual");
  if (currentJob) {
    try {
      if (inTauri) await api.cancelJob(currentJob);
      else mockCancelJob(currentJob);
    } catch {
      /* already finished */
    }
  }
}

/** 清除结果: stop, drop the backend cache and every node's results. */
export async function clearResults() {
  await stopRun();
  if (inTauri) {
    try {
      await api.resetRun();
    } catch (e) {
      toast.error("清除缓存失败", { error: e });
    }
  }
  useGraphStore.getState().resetRuntime();
  useRunStore.getState().setLastError(null);
}
