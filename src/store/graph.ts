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
  // Index signature required by React Flow's Node<Data> constraint.
  [key: string]: unknown;
}

export type FlowNode = Node<FlowNodeData>;

let counter = 0;
const nextId = (prefix: string) => `${prefix}_${counter++}`;

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
}));
