// Graph-aware port helpers shared by connection validation, the port-hover
// suggestion panel, and the AI agent. Pure type/compat logic lives in
// portColors.ts; these read the descriptor + graph stores.
import type { NodeDescriptor, PortType } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";

import { canConnect, paramPortType } from "./portColors";

/** Resolve the data type of a node's port (input or output), including a param
 * that has been promoted to an input handle. */
export function resolvePortType(
  nodeId: string,
  port: string,
  dir: "in" | "out"
): PortType | undefined {
  const n = useGraphStore.getState().nodes.find((x) => x.id === nodeId);
  if (!n) return undefined;
  const d = useDescriptorStore.getState().byId[n.data.descriptorId];
  if (!d) return undefined;
  const list = dir === "in" ? d.inputs : d.outputs;
  const found = list.find((p) => p.name === port)?.type;
  if (found) return found;
  // A promoted parameter accepts a connection of its widget-derived type.
  if (dir === "in") {
    const param = d.params.find((p) => p.name === port);
    if (param) return paramPortType(param.widget);
  }
  return undefined;
}

/** First input port (or promotable param) of `d` that can accept a value of
 * `srcType`. Real inputs are preferred over params. */
export function firstCompatibleInput(
  d: NodeDescriptor,
  srcType: PortType
): { port: string; isParam: boolean } | null {
  for (const p of d.inputs) if (canConnect(srcType, p.type)) return { port: p.name, isParam: false };
  for (const p of d.params)
    if (canConnect(srcType, paramPortType(p.widget))) return { port: p.name, isParam: true };
  return null;
}

/** First output port of `d` that can feed a value into `tgtType`. */
export function firstCompatibleOutput(d: NodeDescriptor, tgtType: PortType): string | null {
  for (const p of d.outputs) if (canConnect(p.type, tgtType)) return p.name;
  return null;
}

/** Match specificity: exact type > any-wildcard > coercion. -1 = incompatible. */
function matchScore(src: PortType, tgt: PortType): number {
  if (!canConnect(src, tgt)) return -1;
  if (src === tgt) return 3;
  if (src === "any" || tgt === "any") return 2;
  return 1;
}

/** Descriptors that can connect to a port of `portType`. `dir='out'` → downstream
 * consumers (their input accepts it); `dir='in'` → upstream producers (their
 * output feeds it). Ranked by match specificity, then display name. */
export function candidateNodes(portType: PortType, dir: "in" | "out"): NodeDescriptor[] {
  const list = useDescriptorStore.getState().list;
  const scored: { d: NodeDescriptor; score: number }[] = [];
  for (const d of list) {
    let best = -1;
    if (dir === "out") {
      for (const p of d.inputs) best = Math.max(best, matchScore(portType, p.type));
      for (const p of d.params)
        best = Math.max(best, matchScore(portType, paramPortType(p.widget)));
    } else {
      for (const p of d.outputs) best = Math.max(best, matchScore(p.type, portType));
    }
    if (best >= 0) scored.push({ d, score: best });
  }
  scored.sort((a, b) => b.score - a.score || a.d.displayName.localeCompare(b.d.displayName));
  return scored.map((s) => s.d);
}
