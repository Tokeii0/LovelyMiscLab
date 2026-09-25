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

import { packedLayout } from "@/flow/layout";

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

// ---------------------------------------------------------------------------
// Undo history
//
// Snapshots hold only what the user built (structure, params, positions) — never
// run results or selection — so undo can't resurrect stale outputs. Every edit
// gets a fresh, never-reused `editRevision`; a snapshot remembers the revision it
// was taken at, so undoing back to the saved state makes the project clean again.
// ---------------------------------------------------------------------------

interface NodeSnap {
  id: string;
  type?: string;
  position: { x: number; y: number };
  data: {
    descriptorId: string;
    label: string;
    color: string;
    params: Record<string, unknown>;
    disabled: boolean;
    inputParams: string[];
  };
}

interface GraphSnapshot {
  nodes: NodeSnap[];
  edges: Edge[];
  rev: number;
}

const MAX_HISTORY = 80;
/** Consecutive edits with the same key within this window merge into one undo step. */
const COALESCE_MS = 800;

let revSeq = 0;
let batchDepth = 0;
let batchStartRev = 0;
let lastKey: string | null = null;
let lastAt = 0;
let dragging = false;

function snapNode(n: FlowNode): NodeSnap {
  return {
    id: n.id,
    type: n.type,
    position: { ...n.position },
    data: {
      descriptorId: n.data.descriptorId,
      label: n.data.label,
      color: n.data.color,
      params: { ...n.data.params },
      disabled: n.data.disabled ?? false,
      inputParams: [...(n.data.inputParams ?? [])],
    },
  };
}

function snapshot(nodes: FlowNode[], edges: Edge[], rev: number): GraphSnapshot {
  return {
    nodes: nodes.map(snapNode),
    edges: edges.map((e) => ({ ...e, selected: false })),
    rev,
  };
}

/** Rebuild nodes from a snapshot, keeping current runtime state + selection by id. */
function restoreNodes(snap: GraphSnapshot, current: FlowNode[]): FlowNode[] {
  const byId = new Map(current.map((n) => [n.id, n]));
  return snap.nodes.map((s) => {
    const cur = byId.get(s.id);
    return {
      id: s.id,
      type: s.type,
      position: { ...s.position },
      selected: cur?.selected ?? false,
      data: {
        ...s.data,
        params: { ...s.data.params },
        inputParams: [...s.data.inputParams],
        status: cur?.data.status ?? "idle",
        progress: cur?.data.progress ?? 0,
        outputs: cur?.data.outputs,
        error: cur?.data.error,
        logs: cur?.data.logs ?? [],
      },
    };
  });
}

function restoreEdges(snap: GraphSnapshot, current: Edge[]): Edge[] {
  const selected = new Set(current.filter((e) => e.selected).map((e) => e.id));
  return snap.edges.map((e) => ({ ...e, selected: selected.has(e.id) }));
}

function pushPast(past: GraphSnapshot[], snap: GraphSnapshot): GraphSnapshot[] {
  return [...past, snap].slice(-MAX_HISTORY);
}

type HistState = { past: GraphSnapshot[]; nodes: FlowNode[]; edges: Edge[]; editRevision: number };

/**
 * State patch for an edit: a new undo step (clearing redo) plus a fresh revision.
 * Inside a batch, or when `key` repeats within COALESCE_MS, the edit merges into
 * the step already on the stack and only the revision moves.
 */
function record(
  state: HistState,
  key?: string
): { past?: GraphSnapshot[]; future?: GraphSnapshot[]; editRevision: number } {
  const editRevision = ++revSeq;
  if (batchDepth > 0) return { editRevision };
  const now = Date.now();
  if (key && key === lastKey && now - lastAt < COALESCE_MS) {
    lastAt = now;
    return { editRevision };
  }
  lastKey = key ?? null;
  lastAt = now;
  return {
    past: pushPast(state.past, snapshot(state.nodes, state.edges, state.editRevision)),
    future: [],
    editRevision,
  };
}

/** Start a fresh undo step on the next edit (e.g. after saving), even mid typing burst. */
export function breakHistoryMerge() {
  lastKey = null;
}

/** Selection flags for "only `ids` selected". */
function selectOnly(nodes: FlowNode[], ids: Set<string>): FlowNode[] {
  return nodes.map((n) => {
    const want = ids.has(n.id);
    return (n.selected ?? false) === want ? n : { ...n, selected: want };
  });
}

function sameJson(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

interface GraphState {
  nodes: FlowNode[];
  edges: Edge[];
  selectedId: string | null;
  past: GraphSnapshot[];
  future: GraphSnapshot[];
  /** Bumped on every change that affects execution (drives live re-runs). */
  runRevision: number;
  /** Unique id of the current graph content; compared against the saved one for "dirty". */
  editRevision: number;

  onNodesChange: (changes: NodeChange<FlowNode>[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (conn: Connection) => void;

  addNode: (descriptor: NodeDescriptor, position: { x: number; y: number }) => string;
  /** `history: false` = silent normalization (no undo step, not a user edit). */
  setParam: (nodeId: string, name: string, value: unknown, opts?: { history?: boolean }) => void;
  setSelected: (id: string | null) => void;
  updateRuntime: (nodeId: string, patch: Partial<FlowNodeData>) => void;
  resetRuntime: () => void;
  clear: () => void;
  deleteNode: (id: string) => void;
  deleteEdge: (id: string) => void;
  /** Delete every selected node/edge as one undo step. Returns false if nothing was selected. */
  deleteSelection: () => boolean;
  duplicateNode: (id: string) => void;
  /** Pack all nodes to fit the current view (vertical-first, wrap horizontally).
   * `aspect` = viewport width/height, so the block matches the screen. */
  arrangeNodes: (aspect: number) => void;
  /** Group the following edits into a single undo step (nestable). */
  beginBatch: () => void;
  endBatch: () => void;
  /** Run `fn` inside a batch. */
  transact: <T>(fn: () => T) => T;
  selectAll: () => void;
  setDisabled: (id: string, disabled: boolean) => void;
  appendLog: (id: string, log: NodeLog) => void;
  toggleParamInput: (nodeId: string, name: string) => void;
  renameNode: (id: string, label: string) => void;
  deselectAll: () => void;
  paste: (clip: Clipboard, dx: number, dy: number) => void;
  /** Replace the graph. `history: "reset"` (open a file) wipes undo; `"record"`
   * (template, AI, MCP) keeps it as one undoable step. `keepRuntime` preserves
   * results of nodes whose descriptor + params didn't change. */
  loadFlow: (
    nodes: SavedNode[],
    edges: SavedEdge[],
    opts?: { history?: "reset" | "record"; keepRuntime?: boolean }
  ) => void;
  undo: () => void;
  redo: () => void;
}

export const useGraphStore = create<GraphState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedId: null,
  past: [],
  future: [],
  runRevision: 0,
  editRevision: 0,

  onNodesChange: (changes) =>
    set((state) => {
      const nodes = applyNodeChanges(changes, state.nodes);
      let structural = false;
      let dragStart = false;
      let keyboardMove = false;
      let moved = false;
      for (const c of changes) {
        if (c.type === "position") {
          moved = true;
          if (c.dragging) {
            if (!dragging) dragStart = true;
            dragging = true;
          } else if (c.dragging === false) {
            dragging = false;
          } else {
            keyboardMove = true; // arrow-key nudge: no drag flag
          }
        } else if (c.type === "remove" || c.type === "add" || c.type === "replace") {
          structural = true;
        }
      }
      if (structural) return { nodes, ...record(state), runRevision: state.runRevision + 1 };
      // A whole drag is one undo step: snapshot once, when it starts.
      if (dragStart) return { nodes, ...record(state) };
      if (keyboardMove) return { nodes, ...record(state, "move") };
      if (moved) return { nodes, editRevision: ++revSeq };
      return { nodes };
    }),
  onEdgesChange: (changes) =>
    set((state) => {
      const edges = applyEdgeChanges(changes, state.edges);
      if (!changes.some((c) => c.type !== "select")) return { edges };
      return { edges, ...record(state), runRevision: state.runRevision + 1 };
    }),
  onConnect: (conn) =>
    set((state) => {
      // An input holds one value: a new wire into an occupied handle replaces the old one.
      const kept = state.edges.filter(
        (e) => !(e.target === conn.target && (e.targetHandle ?? null) === (conn.targetHandle ?? null))
      );
      return {
        edges: addEdge(conn, kept),
        ...record(state),
        runRevision: state.runRevision + 1,
      };
    }),

  addNode: (descriptor, position) => {
    const params: Record<string, unknown> = {};
    for (const p of descriptor.params) params[p.name] = p.default;
    const node: FlowNode = {
      id: nextId(descriptor.id),
      type: flowType(descriptor.id),
      position,
      selected: true,
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
      nodes: [...selectOnly(state.nodes, new Set()), node],
      selectedId: node.id,
      ...record(state),
      runRevision: state.runRevision + 1,
    }));
    return node.id;
  },

  setParam: (nodeId, name, value, opts) =>
    set((state) => {
      const nodes = state.nodes.map((n) =>
        n.id === nodeId
          ? { ...n, data: { ...n.data, params: { ...n.data.params, [name]: value } } }
          : n
      );
      if (opts?.history === false) return { nodes, runRevision: state.runRevision + 1 };
      return {
        nodes,
        // Typing into one field is one undo step, not one per keystroke.
        ...record(state, `param:${nodeId}:${name}`),
        runRevision: state.runRevision + 1,
      };
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

  clear: () =>
    set((state) => {
      if (state.nodes.length === 0 && state.edges.length === 0) return {};
      return {
        nodes: [],
        edges: [],
        selectedId: null,
        ...record(state),
        runRevision: state.runRevision + 1,
      };
    }),

  deleteNode: (id) =>
    set((state) => ({
      nodes: state.nodes.filter((n) => n.id !== id),
      edges: state.edges.filter((e) => e.source !== id && e.target !== id),
      selectedId: state.selectedId === id ? null : state.selectedId,
      ...record(state),
      runRevision: state.runRevision + 1,
    })),

  deleteEdge: (id) =>
    set((state) => ({
      edges: state.edges.filter((e) => e.id !== id),
      ...record(state),
      runRevision: state.runRevision + 1,
    })),

  deleteSelection: () => {
    const { nodes, edges } = get();
    const ids = new Set(nodes.filter((n) => n.selected).map((n) => n.id));
    if (ids.size === 0 && !edges.some((e) => e.selected)) return false;
    set((state) => ({
      nodes: state.nodes.filter((n) => !ids.has(n.id)),
      edges: state.edges.filter((e) => !e.selected && !ids.has(e.source) && !ids.has(e.target)),
      selectedId: state.selectedId && ids.has(state.selectedId) ? null : state.selectedId,
      ...record(state),
      runRevision: state.runRevision + 1,
    }));
    return true;
  },

  duplicateNode: (id) => {
    const n = get().nodes.find((x) => x.id === id);
    if (!n) return;
    const copy: FlowNode = {
      ...n,
      id: nextId(n.data.descriptorId),
      position: { x: n.position.x + 32, y: n.position.y + 32 },
      selected: true,
      data: {
        ...n.data,
        params: { ...n.data.params },
        inputParams: [...(n.data.inputParams ?? [])],
        status: "idle",
        progress: 0,
        outputs: undefined,
        error: undefined,
        logs: [],
      },
    };
    set((state) => ({
      nodes: [...selectOnly(state.nodes, new Set()), copy],
      selectedId: copy.id,
      ...record(state),
      runRevision: state.runRevision + 1,
    }));
  },

  arrangeNodes: (aspect) =>
    set((state) => {
      if (state.nodes.length === 0) return {};
      const pos = packedLayout(state.nodes, state.edges, aspect);
      return {
        nodes: state.nodes.map((n) =>
          pos[n.id] ? { ...n, position: pos[n.id] } : n
        ),
        ...record(state),
      };
    }),

  beginBatch: () =>
    set((state) => {
      if (batchDepth++ > 0) return {};
      // The batch's single undo step is the state before its first edit.
      batchStartRev = state.editRevision;
      lastKey = null;
      return {
        past: pushPast(state.past, snapshot(state.nodes, state.edges, state.editRevision)),
        future: [],
      };
    }),
  endBatch: () =>
    set((state) => {
      if (batchDepth === 0) return {};
      if (--batchDepth > 0) return {};
      // Nothing changed inside the batch: drop the empty undo step again.
      if (state.editRevision === batchStartRev) return { past: state.past.slice(0, -1) };
      return {};
    }),
  transact: (fn) => {
    get().beginBatch();
    try {
      return fn();
    } finally {
      get().endBatch();
    }
  },

  selectAll: () => set({ nodes: get().nodes.map((n) => (n.selected ? n : { ...n, selected: true })) }),

  setDisabled: (id, disabled) =>
    set((state) => ({
      nodes: state.nodes.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, disabled } } : n
      ),
      ...record(state),
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
      ...record(state),
      runRevision: state.runRevision + 1,
    }));
  },

  renameNode: (id, label) =>
    set((state) => ({
      nodes: state.nodes.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, label } } : n
      ),
      ...record(state, `label:${id}`),
    })),

  deselectAll: () =>
    set({
      nodes: selectOnly(get().nodes, new Set()),
      edges: get().edges.map((e) => (e.selected ? { ...e, selected: false } : e)),
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
      nodes: [...selectOnly(state.nodes, new Set()), ...newNodes],
      edges,
      selectedId: newNodes[0]?.id ?? state.selectedId,
      ...record(state),
      runRevision: state.runRevision + 1,
    }));
  },

  loadFlow: (savedNodes, savedEdges, opts = {}) => {
    const { history = "reset", keepRuntime = false } = opts;
    const current = new Map(get().nodes.map((n) => [n.id, n]));
    const flowNodes: FlowNode[] = savedNodes.map((n) => {
      const node: FlowNode = {
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
      };
      const cur = keepRuntime ? current.get(n.id) : undefined;
      if (cur && cur.data.descriptorId === n.descriptorId && sameJson(cur.data.params, n.params)) {
        node.selected = cur.selected;
        node.data = {
          ...node.data,
          status: cur.data.status,
          progress: cur.data.progress,
          outputs: cur.data.outputs,
          error: cur.data.error,
          logs: cur.data.logs,
        };
      }
      return node;
    });
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
    set((state) => {
      const selectedId =
        keepRuntime && state.selectedId && flowNodes.some((n) => n.id === state.selectedId)
          ? state.selectedId
          : null;
      const hist =
        history === "record" ? record(state) : { past: [], future: [], editRevision: ++revSeq };
      lastKey = null;
      return {
        nodes: flowNodes,
        edges: flowEdges,
        selectedId,
        ...hist,
        runRevision: state.runRevision + 1,
      };
    });
  },

  undo: () =>
    set((state) => {
      const previous = state.past[state.past.length - 1];
      if (!previous) return {};
      lastKey = null;
      const nodes = restoreNodes(previous, state.nodes);
      return {
        nodes,
        edges: restoreEdges(previous, state.edges),
        selectedId: nodes.some((n) => n.id === state.selectedId) ? state.selectedId : null,
        past: state.past.slice(0, -1),
        future: [snapshot(state.nodes, state.edges, state.editRevision), ...state.future].slice(
          0,
          MAX_HISTORY
        ),
        editRevision: previous.rev,
        runRevision: state.runRevision + 1,
      };
    }),

  redo: () =>
    set((state) => {
      const next = state.future[0];
      if (!next) return {};
      lastKey = null;
      const nodes = restoreNodes(next, state.nodes);
      return {
        nodes,
        edges: restoreEdges(next, state.edges),
        selectedId: nodes.some((n) => n.id === state.selectedId) ? state.selectedId : null,
        past: pushPast(state.past, snapshot(state.nodes, state.edges, state.editRevision)),
        future: state.future.slice(1),
        editRevision: next.rev,
        runRevision: state.runRevision + 1,
      };
    }),
}));
