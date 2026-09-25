import type { Template } from "@/lib/templates";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";

/**
 * Replace the current graph with a template: mint fresh node ids, apply preset
 * params, and reconnect edges by template-local key. Returns how many nodes were
 * created and which descriptor ids were missing from the registry (skipped).
 */
export function loadTemplate(t: Template): { loaded: number; missing: string[] } {
  const byId = useDescriptorStore.getState().byId;
  // Nothing recognizable (catalog not loaded yet, or an AI result made of unknown
  // nodes): leave the canvas untouched instead of wiping it for an empty graph.
  if (!t.nodes.some((n) => byId[n.descriptorId])) {
    return { loaded: 0, missing: t.nodes.map((n) => n.descriptorId) };
  }
  // The whole replacement is one undo step.
  return useGraphStore.getState().transact(() => buildTemplate(t));
}

function buildTemplate(t: Template): { loaded: number; missing: string[] } {
  const g = useGraphStore.getState();
  const byId = useDescriptorStore.getState().byId;

  g.clear();
  const idMap: Record<string, string> = {};
  const missing: string[] = [];
  let loaded = 0;

  for (const n of t.nodes) {
    const descriptor = byId[n.descriptorId];
    if (!descriptor) {
      missing.push(n.descriptorId);
      continue;
    }
    const id = g.addNode(descriptor, n.position);
    idMap[n.key] = id;
    loaded++;
    if (n.params) {
      for (const [key, value] of Object.entries(n.params)) g.setParam(id, key, value);
    }
  }

  for (const e of t.edges) {
    const source = idMap[e.from.node];
    const target = idMap[e.to.node];
    if (!source || !target) continue;
    g.onConnect({
      source,
      sourceHandle: e.from.port,
      target,
      targetHandle: e.to.port,
    });
  }

  g.deselectAll();
  return { loaded, missing };
}
