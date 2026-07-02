import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
} from "@xyflow/react";
import { create } from "zustand";

import type { NodeDescriptor, PortValue } from "@/lib/types";
import type { SavedEdge, SavedNode } from "@/lib/project";

export type NodeStatus = "idle" | "running" | "done" | "error";

export interface NodeLog {
  time: string;
  level: string;
  message: string;
}

export interface FlowNodeData {
  descriptorId: string;
  label: string;
  color: string;
  params: Record<string, unknown>;
  status: NodeStatus;
  progress: number;
  outputs?: Record<string, PortValue>;
  error?: string;
  disabled?: boolean;
  logs?: NodeLog[];
  /** Param names promoted to input ports (driven by upstream connections). */
  inputParams?: string[];
  // Index signature required by React Flow's Node<Data> constraint.
  [key: string]: unknown;
}

export type FlowNode = Node<FlowNodeData>;

let counter = 0;
const nextId = (prefix: string) => `${prefix}_${counter++}`;

/** React Flow node component to use for a descriptor (most use the generic one). */
const flowType = (descriptorId: string) =>
  descriptorId === "selector" ? "selector" : "generic";

export interface ClipboardNode {
  oldId: string;
  descriptorId: string;
  label: string;
  color: string;
  params: Record<string, unknown>;
  inputParams: string[];
  position: { x: number; y: number };
}
export interface Clipboard {
  nodes: ClipboardNode[];
  edges: {
    source: string;
    sourceHandle?: string | null;
    target: string;
    targetHandle?: string | null;
  }[];
}

interface GraphSnapshot {
  nodes: FlowNode[];
  edges: Edge[];
}

const MAX_HISTORY = 80;

function cloneNode(n: FlowNode): FlowNode {
  return {
    ...n,
    position: { ...n.position },
    data: {
      ...n.data,
      params: { ...n.data.params },
      outputs: n.data.outputs ? { ...n.data.outputs } : undefined,
      logs: n.data.logs ? [...n.data.logs] : undefined,
      inputParams: n.data.inputParams ? [...n.data.inputParams] : undefined,
    },
  };
}

function cloneEdge(e: Edge): Edge {
  return { ...e };
}

function snapshot(nodes: FlowNode[], edges: Edge[]): GraphSnapshot {
  return {
    nodes: nodes.map(cloneNode),
    edges: edges.map(cloneEdge),
  };
}

function pushPast(past: GraphSnapshot[], snap: GraphSnapshot): GraphSnapshot[] {
  return [...past, snap].slice(-MAX_HISTORY);
}

function shouldRecordNodeChanges(changes: NodeChange<FlowNode>[]) {
  return changes.some((c) => c.type !== "select" && c.type !== "dimensions");
}

function shouldRecordEdgeChanges(changes: EdgeChange[]) {
  return changes.some((c) => c.type !== "select");
}

interface GraphState {
  nodes: FlowNode[];
  edges: Edge[];
  selectedId: string | null;
  past: GraphSnapshot[];
  future: GraphSnapshot[];
  runRevision: number;

  onNodesChange: (changes: NodeChange<FlowNode>[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (conn: Connection) => void;

  addNode: (descriptor: NodeDescriptor, position: { x: number; y: number }) => string;
  setParam: (nodeId: string, name: string, value: unknown) => void;
  setSelected: (id: string | null) => void;
  updateRuntime: (nodeId: string, patch: Partial<FlowNodeData>) => void;
  resetRuntime: () => void;
  clear: () => void;
  deleteNode: (id: string) => void;
  deleteEdge: (id: string) => void;
  duplicateNode: (id: string) => void;
  selectAll: () => void;
  setDisabled: (id: string, disabled: boolean) => void;
  appendLog: (id: string, log: NodeLog) => void;
  toggleParamInput: (nodeId: string, name: string) => void;
  renameNode: (id: string, label: string) => void;
  deselectAll: () => void;
  paste: (clip: Clipboard, dx: number, dy: number) => void;
  loadFlow: (nodes: SavedNode[], edges: SavedEdge[]) => void;
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
}

export const useGraphStore = create<GraphState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedId: null,
  past: [],
  future: [],
  runRevision: 0,

  onNodesChange: (changes) =>
    set((state) => {
      const nodes = applyNodeChanges(changes, state.nodes);
      if (!shouldRecordNodeChanges(changes)) return { nodes };
      return {
        nodes,
        past: pushPast(state.past, snapshot(state.nodes, state.edges)),
        future: [],
      };
    }),
  onEdgesChange: (changes) =>
    set((state) => {
      const edges = applyEdgeChanges(changes, state.edges);
      if (!shouldRecordEdgeChanges(changes)) return { edges };
      return {
        edges,
        past: pushPast(state.past, snapshot(state.nodes, state.edges)),
        future: [],
        runRevision: state.runRevision + 1,
      };
    }),
  onConnect: (conn) =>
    set((state) => ({
      edges: addEdge(conn, state.edges),
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    })),

  addNode: (descriptor, position) => {
    const params: Record<string, unknown> = {};
    for (const p of descriptor.params) params[p.name] = p.default;
    const node: FlowNode = {
      id: nextId(descriptor.id),
      type: flowType(descriptor.id),
      position,
      data: {
        descriptorId: descriptor.id,
        label: descriptor.displayName,
        color: descriptor.color,
        params,
        status: "idle",
        progress: 0,
        disabled: false,
        logs: [],
        inputParams: [],
      },
    };
    set((state) => ({
      nodes: [...state.nodes, node],
      selectedId: node.id,
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    }));
    return node.id;
  },

  setParam: (nodeId, name, value) =>
    set((state) => ({
      nodes: state.nodes.map((n) =>
        n.id === nodeId
          ? {
              ...n,
              data: { ...n.data, params: { ...n.data.params, [name]: value } },
            }
          : n
      ),
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    })),

  setSelected: (id) => set({ selectedId: id }),

  updateRuntime: (nodeId, patch) =>
    set({
      nodes: get().nodes.map((n) =>
        n.id === nodeId ? { ...n, data: { ...n.data, ...patch } } : n
      ),
    }),

  resetRuntime: () =>
    set({
      nodes: get().nodes.map((n) => ({
        ...n,
        data: {
          ...n.data,
          status: "idle",
          progress: 0,
          error: undefined,
          outputs: undefined,
          logs: [],
        },
      })),
    }),

  clear: () =>
    set((state) => ({
      nodes: [],
      edges: [],
      selectedId: null,
      past: state.nodes.length || state.edges.length
        ? pushPast(state.past, snapshot(state.nodes, state.edges))
        : state.past,
      future: [],
      runRevision: state.runRevision + 1,
    })),

  deleteNode: (id) =>
    set((state) => ({
      nodes: state.nodes.filter((n) => n.id !== id),
      edges: state.edges.filter((e) => e.source !== id && e.target !== id),
      selectedId: state.selectedId === id ? null : state.selectedId,
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    })),

  deleteEdge: (id) =>
    set((state) => ({
      edges: state.edges.filter((e) => e.id !== id),
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    })),

  duplicateNode: (id) => {
    const n = get().nodes.find((x) => x.id === id);
    if (!n) return;
    const copy: FlowNode = {
      ...n,
      id: nextId(n.data.descriptorId),
      position: { x: n.position.x + 32, y: n.position.y + 32 },
      selected: false,
      data: {
        ...n.data,
        params: { ...n.data.params },
        inputParams: [...(n.data.inputParams ?? [])],
        status: "idle",
        progress: 0,
        outputs: undefined,
        error: undefined,
      },
    };
    set((state) => ({
      nodes: [...state.nodes, copy],
      selectedId: copy.id,
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    }));
  },

  selectAll: () => set({ nodes: get().nodes.map((n) => ({ ...n, selected: true })) }),

  setDisabled: (id, disabled) =>
    set((state) => ({
      nodes: state.nodes.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, disabled } } : n
      ),
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    })),

  appendLog: (id, log) =>
    set({
      nodes: get().nodes.map((n) =>
        n.id === id
          ? { ...n, data: { ...n.data, logs: [...(n.data.logs ?? []), log] } }
          : n
      ),
    }),

  toggleParamInput: (nodeId, name) => {
    const node = get().nodes.find((n) => n.id === nodeId);
    const removing = node?.data.inputParams?.includes(name) ?? false;
    set((state) => ({
      nodes: state.nodes.map((n) =>
        n.id === nodeId
          ? {
              ...n,
              data: {
                ...n.data,
                inputParams: removing
                  ? (n.data.inputParams ?? []).filter((x) => x !== name)
                  : [...(n.data.inputParams ?? []), name],
              },
            }
          : n
      ),
      // Un-promoting drops any edge that was feeding this param.
      edges: removing
        ? state.edges.filter((e) => !(e.target === nodeId && e.targetHandle === name))
        : state.edges,
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    }));
  },

  renameNode: (id, label) =>
    set((state) => ({
      nodes: state.nodes.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, label } } : n
      ),
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
    })),

  deselectAll: () =>
    set({
      nodes: get().nodes.map((n) => (n.selected ? { ...n, selected: false } : n)),
      selectedId: null,
    }),

  paste: (clip, dx, dy) => {
    const idMap: Record<string, string> = {};
    const newNodes: FlowNode[] = clip.nodes.map((cn) => {
      const id = nextId(cn.descriptorId);
      idMap[cn.oldId] = id;
      return {
        id,
        type: flowType(cn.descriptorId),
        position: { x: cn.position.x + dx, y: cn.position.y + dy },
        selected: true,
        data: {
          descriptorId: cn.descriptorId,
          label: cn.label,
          color: cn.color,
          params: { ...cn.params },
          status: "idle",
          progress: 0,
          disabled: false,
          logs: [],
          inputParams: [...cn.inputParams],
        },
      };
    });
    let edges = get().edges;
    for (const e of clip.edges) {
      const source = idMap[e.source];
      const target = idMap[e.target];
      if (source && target) {
        edges = addEdge(
          {
            source,
            sourceHandle: e.sourceHandle ?? null,
            target,
            targetHandle: e.targetHandle ?? null,
          },
          edges
        );
      }
    }
    set((state) => ({
      nodes: [...state.nodes.map((n) => ({ ...n, selected: false })), ...newNodes],
      edges,
      selectedId: newNodes[0]?.id ?? get().selectedId,
      past: pushPast(state.past, snapshot(state.nodes, state.edges)),
      future: [],
      runRevision: state.runRevision + 1,
    }));
  },

  loadFlow: (savedNodes, savedEdges) => {
    const flowNodes: FlowNode[] = savedNodes.map((n) => ({
      id: n.id,
      type: flowType(n.descriptorId),
      position: { x: n.position.x, y: n.position.y },
      data: {
        descriptorId: n.descriptorId,
        label: n.label,
        color: n.color,
        params: { ...n.params },
        status: "idle",
        progress: 0,
        disabled: n.disabled ?? false,
        logs: [],
        inputParams: [...(n.inputParams ?? [])],
      },
    }));
    const flowEdges: Edge[] = savedEdges.map((e) => ({
      id: e.id,
      source: e.source,
      sourceHandle: e.sourceHandle ?? null,
      target: e.target,
      targetHandle: e.targetHandle ?? null,
      ...(e.type ? { type: e.type } : {}),
    }));
    // Bump the id counter past loaded numeric suffixes to avoid future collisions.
    let max = counter;
    for (const n of savedNodes) {
      const m = /_(\d+)$/.exec(n.id);
      if (m) max = Math.max(max, Number(m[1]) + 1);
    }
    counter = max;
    set({
      nodes: flowNodes,
      edges: flowEdges,
      selectedId: null,
      past: [],
      future: [],
      runRevision: get().runRevision + 1,
    });
  },

  undo: () =>
    set((state) => {
      const previous = state.past[state.past.length - 1];
      if (!previous) return {};
      return {
        nodes: previous.nodes.map(cloneNode),
        edges: previous.edges.map(cloneEdge),
        selectedId: null,
        past: state.past.slice(0, -1),
        future: [snapshot(state.nodes, state.edges), ...state.future].slice(0, MAX_HISTORY),
        runRevision: state.runRevision + 1,
      };
    }),

  redo: () =>
    set((state) => {
      const next = state.future[0];
      if (!next) return {};
      return {
        nodes: next.nodes.map(cloneNode),
        edges: next.edges.map(cloneEdge),
        selectedId: null,
        past: pushPast(state.past, snapshot(state.nodes, state.edges)),
        future: state.future.slice(1),
        runRevision: state.runRevision + 1,
      };
    }),

  canUndo: () => get().past.length > 0,
  canRedo: () => get().future.length > 0,
}));
