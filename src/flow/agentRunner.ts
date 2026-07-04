import type { ReactFlowInstance } from "@xyflow/react";

import { api, type AgentEvent } from "@/lib/bindings";
import { useAgentStore } from "@/store/agent";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";

import { viewportAspect } from "./layout";

type ConnectEvent = Extract<AgentEvent, { kind: "connect" }>;

/** Pause between visible beats — the "watch it think" pace. */
const STEP_MS = 360;

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** Cascade placement while building; the final arrange re-lays everything. */
function placeDuringBuild(n: number): { x: number; y: number } {
  return { x: 60 + (n % 6) * 240, y: 60 + Math.floor(n / 6) * 170 };
}

/**
 * Run the AI agent for `prompt` and replay its streamed steps as a paced,
 * one-node-at-a-time build: each node is placed, then the edges that belong to
 * it fire right after it (pulled forward regardless of how the model batched the
 * tool calls), the camera follows, and every step shows the agent's one-line
 * 巧思. Clears the canvas first — this is a fresh build.
 */
export async function runAgent(
  task: string,
  data: string | null,
  rf: ReactFlowInstance
): Promise<void> {
  const byId = useDescriptorStore.getState().byId;
  const idMap: Record<string, string> = {};
  const placedKeys = new Set<string>();
  const queue: AgentEvent[] = []; // streamed events awaiting paced processing
  const deferred: ConnectEvent[] = []; // edges dequeued before both endpoints exist
  let placed = 0;
  let streamEnded = false;
  let completed = false;
  let dataInjected = false;

  // When the user supplies a long input separately, the app fills it into the
  // source node itself — the model only gets a short preview (to pick the first
  // decode step) and is told NOT to echo the full blob (which would blow the
  // output-token limit and truncate the JSON).
  const prompt =
    data && data.trim()
      ? `${task.trim()}\n\n【待处理数据】用户已单独提供一段待处理数据（共 ${data.length} 字），` +
        `程序会自动把它填入源节点 text_input 的 text 参数。你搭图时：把 text_input 的 text 留空，` +
        `**绝不要**在输出里重复这段完整数据（否则会超长截断）。开头预览（仅供你判断编码类型，勿照抄）：\n` +
        `${data.slice(0, 200)}${data.length > 200 ? " …（后略）" : ""}`
      : task;

  useGraphStore.getState().clear();
  // Collapse the whole build into one undo entry (the pre-build snapshot).
  useGraphStore.getState().setSuppressHistory(true);
  useAgentStore.getState().start();

  const follow = (pos: { x: number; y: number }) =>
    rf.setCenter(pos.x + 100, pos.y + 40, { zoom: rf.getViewport().zoom, duration: 300 });

  const labelOf = (key: string): string => {
    const id = idMap[key];
    if (!id) return key;
    const n = useGraphStore.getState().nodes.find((x) => x.id === id);
    const d = n ? byId[n.data.descriptorId] : undefined;
    return d?.displayName ?? key;
  };

  const ready = (c: ConnectEvent) => placedKeys.has(c.fromKey) && placedKeys.has(c.toKey);

  const applyConnect = (ev: ConnectEvent) => {
    const gg = useGraphStore.getState();
    const source = idMap[ev.fromKey];
    const target = idMap[ev.toKey];
    if (!source || !target) return;
    // Promote a param target to an input handle first, so the edge has a handle.
    const tNode = gg.nodes.find((n) => n.id === target);
    const td = tNode ? byId[tNode.data.descriptorId] : undefined;
    const isInput = td?.inputs.some((p) => p.name === ev.toPort);
    const isParam = !isInput && !!td?.params.some((p) => p.name === ev.toPort);
    if (isParam) gg.toggleParamInput(target, ev.toPort);
    gg.onConnect({ source, sourceHandle: ev.fromPort, target, targetHandle: ev.toPort });
  };

  /** Apply one event. Returns true if it was a visible "beat" worth pausing on. */
  const applyStep = (ev: AgentEvent): boolean => {
    const a = useAgentStore.getState();
    const gg = useGraphStore.getState();
    switch (ev.kind) {
      case "started":
        a.setJob(ev.job);
        return false;
      case "thinking":
        a.pushStep({ kind: "thinking", text: ev.text });
        return true;
      case "addNode": {
        const d = byId[ev.descriptorId];
        if (!d) {
          a.pushStep({ kind: "error", text: `未知节点 ${ev.descriptorId}`, ok: false });
          return false;
        }
        const pos = placeDuringBuild(placed++);
        const realId = gg.addNode(d, pos);
        idMap[ev.key] = realId;
        placedKeys.add(ev.key);
        if (ev.params && typeof ev.params === "object") {
          for (const [k, v] of Object.entries(ev.params as Record<string, unknown>)) {
            gg.setParam(realId, k, v);
          }
        }
        // Inject the user's long input into the first text source, overriding
        // whatever (empty/placeholder) text the model produced.
        if (data && !dataInjected && d.inputs.length === 0 && d.params.some((p) => p.name === "text")) {
          gg.setParam(realId, "text", data);
          dataInjected = true;
          a.pushStep({ kind: "param", text: `⚙ 已填入数据 → ${d.displayName}`, detail: `${data.length} 字` });
        }
        gg.setSelected(realId);
        follow(pos);
        a.pushStep({ kind: "add", text: `＋ ${d.displayName}`, detail: ev.reason || undefined });
        // Interleave: pull this node's edges — from the deferred list or from
        // later in the batch — to the front so they fire right after it.
        for (let i = deferred.length - 1; i >= 0; i--) {
          if (ready(deferred[i])) queue.unshift(deferred.splice(i, 1)[0]);
        }
        for (let i = queue.length - 1; i >= 0; i--) {
          const c = queue[i];
          if (c.kind === "connect" && ready(c)) {
            queue.splice(i, 1);
            queue.unshift(c);
          }
        }
        return true;
      }
      case "connect": {
        if (!ready(ev)) {
          deferred.push(ev); // wait until both endpoints are placed
          return false;
        }
        applyConnect(ev);
        a.pushStep({
          kind: "connect",
          text: `${labelOf(ev.fromKey)} → ${labelOf(ev.toKey)}`,
          detail: ev.reason || undefined,
        });
        return true;
      }
      case "setParam": {
        const id = idMap[ev.key];
        if (id) gg.setParam(id, ev.name, ev.value);
        if (ev.reason) {
          a.pushStep({ kind: "param", text: `⚙ ${ev.name}`, detail: ev.reason });
          return true;
        }
        return false;
      }
      case "runStart":
        for (const k of ev.keys) {
          const id = idMap[k];
          if (id) gg.updateRuntime(id, { status: "running" });
        }
        a.pushStep({ kind: "run", text: `▶ 运行 ${ev.keys.length} 个节点` });
        return true;
      case "nodeResult": {
        const id = idMap[ev.key];
        if (id) gg.updateRuntime(id, { status: ev.ok ? "done" : "error" });
        a.pushStep({ kind: "result", text: `= ${labelOf(ev.key)}: ${ev.summary}`, ok: ev.ok });
        return true;
      }
      case "toolError":
        a.pushStep({ kind: "error", text: `${ev.tool}: ${ev.message}`, ok: false });
        return true;
      case "done":
        completed = true;
        a.pushStep({ kind: "done", text: ev.notes || "完成" });
        a.finish(ev.notes || "完成");
        return false;
      case "error":
        a.pushStep({ kind: "error", text: ev.message, ok: false });
        a.setError(ev.message);
        return false;
    }
    return false;
  };

  const runPromise = api
    .agentRun(prompt, (ev) => queue.push(ev))
    .catch((e) => useAgentStore.getState().setError(String(e)))
    .finally(() => {
      streamEnded = true;
    });

  try {
    for (;;) {
      if (queue.length === 0) {
        if (!streamEnded) {
          await sleep(50); // wait for more streamed events
          continue;
        }
        // Stream ended: flush any deferred edges that are now satisfiable.
        const flush = deferred.filter(ready);
        if (flush.length === 0) break;
        for (const c of flush) {
          deferred.splice(deferred.indexOf(c), 1);
          queue.push(c);
        }
      }
      const ev = queue.shift()!;
      const beat = applyStep(ev);
      // Pace only while still running (cancel/finish drains the rest instantly).
      if (beat && useAgentStore.getState().running) await sleep(STEP_MS);
    }
    await runPromise;
  } finally {
    useGraphStore.getState().setSuppressHistory(false);
    const gg = useGraphStore.getState();
    if (completed && gg.nodes.length > 0) {
      gg.arrangeNodes(viewportAspect());
      requestAnimationFrame(() => rf.fitView({ duration: 300, padding: 0.15 }));
    }
  }
}
