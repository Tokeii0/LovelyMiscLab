import type { ReactFlowInstance } from "@xyflow/react";

import { api, type AgentEvent } from "@/lib/bindings";
import { useAgentStore } from "@/store/agent";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";

import { viewportAspect } from "./layout";

/** Cascade placement while building (nodes arrive before their edges). The final
 * `arrangeNodes` on `done` re-lays everything into proper layers, so this only
 * needs to look reasonable mid-stream. */
function placeDuringBuild(n: number): { x: number; y: number } {
  return { x: 60 + (n % 6) * 240, y: 60 + Math.floor(n / 6) * 170 };
}

/** Run the AI agent for `prompt`, applying each streamed step to the canvas live
 * (camera follows the newest node) and logging it to the agent panel. Clears the
 * canvas first — this is a fresh build. */
export async function runAgent(prompt: string, rf: ReactFlowInstance): Promise<void> {
  const g = useGraphStore.getState();
  const byId = useDescriptorStore.getState().byId;
  const idMap: Record<string, string> = {};
  let placed = 0;

  g.clear();
  // Collapse the whole build into one undo entry: the pre-build snapshot pushed
  // by clear() above. Every step after this records no history.
  useGraphStore.getState().setSuppressHistory(true);
  useAgentStore.getState().start();

  const follow = (pos: { x: number; y: number }) =>
    rf.setCenter(pos.x + 100, pos.y + 40, { zoom: rf.getViewport().zoom, duration: 250 });

  const handle = (ev: AgentEvent) => {
    const a = useAgentStore.getState();
    const gg = useGraphStore.getState();
    switch (ev.kind) {
      case "started":
        a.setJob(ev.job);
        break;
      case "thinking":
        a.pushStep({ kind: "thinking", text: ev.text });
        break;
      case "addNode": {
        const d = byId[ev.descriptorId];
        if (!d) {
          a.pushStep({ kind: "error", text: `未知节点 ${ev.descriptorId}`, ok: false });
          break;
        }
        const pos = placeDuringBuild(placed++);
        const realId = gg.addNode(d, pos);
        idMap[ev.key] = realId;
        if (ev.params && typeof ev.params === "object") {
          for (const [k, v] of Object.entries(ev.params as Record<string, unknown>)) {
            gg.setParam(realId, k, v);
          }
        }
        gg.setSelected(realId);
        follow(pos);
        a.pushStep({ kind: "add", text: `+ ${d.displayName}` });
        break;
      }
      case "connect": {
        const source = idMap[ev.fromKey];
        const target = idMap[ev.toKey];
        if (!source || !target) {
          a.pushStep({ kind: "error", text: `连线失败 ${ev.fromKey}→${ev.toKey}`, ok: false });
          break;
        }
        // If the target handle is a param (not a declared input), promote it to
        // an input port first so the connection point exists.
        const tNode = gg.nodes.find((n) => n.id === target);
        const td = tNode ? byId[tNode.data.descriptorId] : undefined;
        const isDeclaredInput = td?.inputs.some((p) => p.name === ev.toPort);
        const isParam = !isDeclaredInput && !!td?.params.some((p) => p.name === ev.toPort);
        if (isParam) gg.toggleParamInput(target, ev.toPort);
        gg.onConnect({
          source,
          sourceHandle: ev.fromPort,
          target,
          targetHandle: ev.toPort,
        });
        a.pushStep({ kind: "connect", text: `⇄ ${ev.fromKey} → ${ev.toKey}` });
        break;
      }
      case "setParam": {
        const id = idMap[ev.key];
        if (id) gg.setParam(id, ev.name, ev.value);
        break;
      }
      case "runStart":
        for (const k of ev.keys) {
          const id = idMap[k];
          if (id) gg.updateRuntime(id, { status: "running" });
        }
        a.pushStep({ kind: "run", text: `▶ 运行 ${ev.keys.length} 个节点` });
        break;
      case "nodeResult": {
        const id = idMap[ev.key];
        if (id) gg.updateRuntime(id, { status: ev.ok ? "done" : "error" });
        a.pushStep({ kind: "result", text: `= ${ev.key}: ${ev.summary}`, ok: ev.ok });
        break;
      }
      case "toolError":
        a.pushStep({ kind: "error", text: `${ev.tool}: ${ev.message}`, ok: false });
        break;
      case "done":
        a.pushStep({ kind: "done", text: ev.notes || "完成" });
        a.finish(ev.notes || "完成");
        gg.arrangeNodes(viewportAspect());
        requestAnimationFrame(() => rf.fitView({ duration: 300, padding: 0.15 }));
        break;
      case "error":
        a.pushStep({ kind: "error", text: ev.message, ok: false });
        a.setError(ev.message);
        break;
    }
  };

  try {
    await api.agentRun(prompt, handle);
  } catch (e) {
    useAgentStore.getState().setError(String(e));
  } finally {
    useGraphStore.getState().setSuppressHistory(false);
  }
}
