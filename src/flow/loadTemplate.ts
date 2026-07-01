import type { Template } from "@/lib/templates";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";

/**
 * Replace the current graph with a template: mint fresh node ids, apply preset
 * params, and reconnect edges by template-local key. Returns how many nodes were
 * actually created (descriptors missing from the registry are skipped).
 */
export function loadTemplate(t: Template): number {
  const g = useGraphStore.getState();
  const byId = useDescriptorStore.getState().byId;

  g.clear();
  const idMap: Record<string, string> = {};
  let loaded = 0;

  for (const n of t.nodes) {
    const descriptor = byId[n.descriptorId];
    if (!descriptor) {
      console.warn(`模板「${t.name}」缺少节点: ${n.descriptorId}`);
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

  g.setSelected(null);
  return loaded;
}
