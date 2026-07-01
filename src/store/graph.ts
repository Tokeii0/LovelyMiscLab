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

interface GraphState {
  nodes: FlowNode[];
  edges: Edge[];
  selectedId: string | null;

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
}

export const useGraphStore = create<GraphState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedId: null,

  onNodesChange: (changes) =>
    set({ nodes: applyNodeChanges(changes, get().nodes) }),
  onEdgesChange: (changes) =>
    set({ edges: applyEdgeChanges(changes, get().edges) }),
  onConnect: (conn) => set({ edges: addEdge(conn, get().edges) }),

  addNode: (descriptor, position) => {
    const params: Record<string, unknown> = {};
    for (const p of descriptor.params) params[p.name] = p.default;
    const node: FlowNode = {
      id: nextId(descriptor.id),
      type: "generic",
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
    set({ nodes: [...get().nodes, node], selectedId: node.id });
    return node.id;
  },

  setParam: (nodeId, name, value) =>
    set({
      nodes: get().nodes.map((n) =>
        n.id === nodeId
          ? {
              ...n,
              data: { ...n.data, params: { ...n.data.params, [name]: value } },
            }
          : n
      ),
    }),

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

  clear: () => set({ nodes: [], edges: [], selectedId: null }),

  deleteNode: (id) =>
    set({
      nodes: get().nodes.filter((n) => n.id !== id),
      edges: get().edges.filter((e) => e.source !== id && e.target !== id),
      selectedId: get().selectedId === id ? null : get().selectedId,
    }),

  deleteEdge: (id) => set({ edges: get().edges.filter((e) => e.id !== id) }),

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
    set({ nodes: [...get().nodes, copy], selectedId: copy.id });
  },

  selectAll: () => set({ nodes: get().nodes.map((n) => ({ ...n, selected: true })) }),

  setDisabled: (id, disabled) =>
    set({
      nodes: get().nodes.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, disabled } } : n
      ),
    }),

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
    set({
      nodes: get().nodes.map((n) =>
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
        ? get().edges.filter((e) => !(e.target === nodeId && e.targetHandle === name))
        : get().edges,
    });
  },

  renameNode: (id, label) =>
    set({
      nodes: get().nodes.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, label } } : n
      ),
    }),

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
        type: "generic",
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
    set({
      nodes: [...get().nodes.map((n) => ({ ...n, selected: false })), ...newNodes],
      edges,
      selectedId: newNodes[0]?.id ?? get().selectedId,
    });
  },
}));
