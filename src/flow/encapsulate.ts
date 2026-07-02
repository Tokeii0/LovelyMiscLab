// Turn the currently-selected canvas nodes into a composite module: build the
// inner sub-graph and auto-detect its boundary (dangling) input/output ports.

import type {
  BoundaryPort,
  CompositeModule,
  NodeDescriptor,
  ParamWidget,
  PortType,
  SerializedGraph,
} from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";

/** A candidate boundary port surfaced in the create dialog (user can rename/drop). */
export interface DetectedPort {
  key: string; // stable React key: "<nodeId>:<port>"
  label: string; // editable display label
  portType: PortType;
  node: string; // inner node id
  port: string; // inner port name
  include: boolean;
}

export interface Encapsulation {
  graph: SerializedGraph;
  inputs: DetectedPort[];
  outputs: DetectedPort[];
}

function paramType(w: ParamWidget): PortType {
  if (w.kind === "number" || w.kind === "slider") return "number";
  if (w.kind === "toggle") return "bool";
  return "text";
}

/** Read the selection and derive the inner graph + dangling boundary ports. */
export function buildEncapsulation(): Encapsulation | null {
  const g = useGraphStore.getState();
  const byId = useDescriptorStore.getState().byId;
  const sel = g.nodes.filter((n) => n.selected && !n.data.disabled);
  if (sel.length === 0) return null;
  const ids = new Set(sel.map((n) => n.id));

  const innerEdges = g.edges.filter(
    (e) => e.sourceHandle && e.targetHandle && ids.has(e.source) && ids.has(e.target)
  );
  const graph: SerializedGraph = {
    nodes: sel.map((n) => ({
      id: n.id,
      descriptorId: n.data.descriptorId,
      params: n.data.params,
      position: [n.position.x, n.position.y],
    })),
    edges: innerEdges.map((e) => ({
      from: { node: e.source, port: e.sourceHandle as string },
      to: { node: e.target, port: e.targetHandle as string },
    })),
  };

  const fed = new Set(innerEdges.map((e) => `${e.target}:${e.targetHandle}`));
  const consumed = new Set(innerEdges.map((e) => `${e.source}:${e.sourceHandle}`));

  const inputs: DetectedPort[] = [];
  const outputs: DetectedPort[] = [];
  for (const n of sel) {
    const d = byId[n.data.descriptorId];
    if (!d) continue;
    const promoted = new Set(n.data.inputParams ?? []);
    // Declared inputs + any params promoted to input ports.
    const inPorts = [
      ...d.inputs.map((p) => ({ name: p.name, label: p.label, type: p.type })),
      ...d.params
        .filter((p) => promoted.has(p.name))
        .map((p) => ({ name: p.name, label: p.label, type: paramType(p.widget) })),
    ];
    for (const p of inPorts) {
      if (!fed.has(`${n.id}:${p.name}`)) {
        inputs.push({
          key: `${n.id}:${p.name}`,
          label: `${n.data.label} · ${p.label}`,
          portType: p.type,
          node: n.id,
          port: p.name,
          include: true,
        });
      }
    }
    for (const p of d.outputs) {
      if (!consumed.has(`${n.id}:${p.name}`)) {
        outputs.push({
          key: `${n.id}:${p.name}`,
          label: `${n.data.label} · ${p.label}`,
          portType: p.type,
          node: n.id,
          port: p.name,
          include: true,
        });
      }
    }
  }
  return { graph, inputs, outputs };
}

function slugify(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "");
}

/** Assign unique handle names (from labels) to the included ports. */
export function toBoundaryPorts(ports: DetectedPort[]): BoundaryPort[] {
  const used = new Set<string>();
  return ports
    .filter((p) => p.include)
    .map((p) => {
      const base = slugify(p.label) || p.port;
      let name = base;
      let i = 2;
      while (used.has(name)) name = `${base}_${i++}`;
      used.add(name);
      return { name, label: p.label, portType: p.portType, node: p.node, port: p.port };
    });
}

export function newModuleId(name: string): string {
  const slug = slugify(name) || "module";
  return `mod_${slug}_${Math.random().toString(36).slice(2, 7)}`;
}

/** The palette descriptor for a composite (mirrors the Rust `CompositeModule::descriptor`). */
export function compositeDescriptor(m: CompositeModule): NodeDescriptor {
  const port = (b: BoundaryPort) => ({ name: b.name, label: b.label, type: b.portType, required: false });
  return {
    id: m.id,
    category: m.category || "自定义",
    displayName: m.name,
    description: m.description,
    color: m.color || "#8b5cf6",
    inputs: m.inputs.map(port),
    outputs: m.outputs.map(port),
    params: [],
    cost: "medium",
  };
}
