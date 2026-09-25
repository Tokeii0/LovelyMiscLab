import { create } from "zustand";

import type { PortValue, ProgressMsg } from "@/lib/types";

/** manual = runs only when asked; live = re-run automatically after every edit. */
export type RunMode = "manual" | "live";
export type RunStatus = "running" | "success" | "error" | "cancelled";

export interface RunHistoryNode {
  id: string;
  label: string;
  descriptorId: string;
  status: string;
  error?: string;
  /** Snapshot of the node's output ports (size-limited for storage). */
  outputs?: Record<string, PortValue>;
}

export interface RunHistoryEvent {
  time: string;
  message: string;
  level: "info" | "success" | "warn" | "error";
  node?: string;
}

export interface RunHistoryEntry {
  id: string;
  scope: "graph" | "node" | "debug";
  title: string;
  projectName: string;
  startedAt: string;
  finishedAt?: string;
  elapsed: number;
  status: RunStatus;
  nodeCount: number;
  edgeCount: number;
  error?: string;
  nodes: RunHistoryNode[];
  events: RunHistoryEvent[];
}

const HISTORY_KEY = "misclab-run-history-v1";
const MAX_HISTORY = 80;

function loadHistory(): RunHistoryEntry[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as RunHistoryEntry[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function persistHistory(entries: RunHistoryEntry[]) {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(entries.slice(0, MAX_HISTORY)));
  } catch {
    /* ignore storage failures */
  }
}

function eventFromProgress(msg: ProgressMsg): RunHistoryEvent | null {
  const time = new Date().toLocaleTimeString();
  switch (msg.kind) {
    case "jobStarted":
      return { time, level: "info", message: `任务已启动：${msg.job}` };
    case "nodeEntered":
      return { time, level: "info", node: msg.node, message: "节点开始执行" };
    case "nodeProgress":
      return null; // too chatty for the timeline; the node shows live progress
    case "nodeDone":
      return {
        time,
        level: "success",
        node: msg.node,
        message: msg.cached ? "节点结果来自缓存" : "节点执行成功",
      };
    case "nodeSkipped":
      return { time, level: "warn", node: msg.node, message: msg.reason };
    case "nodeFailed":
      return { time, level: "error", node: msg.node, message: msg.error };
    case "log":
      return {
        time,
        level: msg.level === "error" ? "error" : msg.level === "warn" ? "warn" : "info",
        node: msg.node ?? undefined,
        message: msg.message,
      };
    case "jobDone":
      return { time, level: "success", message: `任务完成：${msg.job}` };
    case "jobFailed":
      return { time, level: "error", message: msg.error };
  }
}

/** What is executing right now (drives the title-bar status). */
export interface ActiveRun {
  kind: "graph" | "toNode" | "node" | "live";
  title: string;
}

interface RunState {
  mode: RunMode;
  /** True while a run is in flight. */
  running: boolean;
  activeRun: ActiveRun | null;
  /** Duration of the last run, in ms. */
  elapsed: number;
  /** Most recent graph-level error, shown when no node captured it. */
  lastError: string | null;
  /** Persisted run timeline, newest first. */
  history: RunHistoryEntry[];
  activeHistoryId: string | null;
  setMode: (m: RunMode) => void;
  setRunning: (r: boolean, active?: ActiveRun | null) => void;
  setElapsed: (ms: number) => void;
  setLastError: (error: string | null) => void;
  /** `replaceId`: reuse that entry (live mode keeps one rolling "实时运行" record). */
  startHistory: (
    entry: Omit<RunHistoryEntry, "id" | "startedAt" | "elapsed" | "status" | "events">,
    replaceId?: string | null
  ) => string;
  recordProgress: (msg: ProgressMsg) => void;
  appendHistoryEvent: (entryId: string, event: RunHistoryEvent) => void;
  finishHistory: (
    entryId: string,
    patch: Partial<Pick<RunHistoryEntry, "status" | "elapsed" | "error" | "nodes">>
  ) => void;
  removeHistory: (entryId: string) => void;
  clearHistory: () => void;
}

export const useRunStore = create<RunState>((set) => ({
  mode: "manual",
  running: false,
  activeRun: null,
  elapsed: 0,
  lastError: null,
  history: loadHistory(),
  activeHistoryId: null,
  setMode: (mode) => set({ mode }),
  setRunning: (running, active) => set({ running, activeRun: running ? (active ?? null) : null }),
  setElapsed: (elapsed) => set({ elapsed }),
  setLastError: (lastError) => set({ lastError }),
  startHistory: (entry, replaceId) => {
    const id = replaceId ?? `run_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
    const full: RunHistoryEntry = {
      ...entry,
      id,
      startedAt: new Date().toISOString(),
      elapsed: 0,
      status: "running",
      events: [],
    };
    set((state) => {
      const history = [full, ...state.history.filter((e) => e.id !== id)].slice(0, MAX_HISTORY);
      persistHistory(history);
      return { history, activeHistoryId: id };
    });
    return id;
  },
  // Events accumulate in memory; storage is written when the run finishes, not
  // on every progress message (which re-serialized the whole history each time).
  recordProgress: (msg) => {
    const event = eventFromProgress(msg);
    if (!event) return;
    set((state) => {
      if (!state.activeHistoryId) return {};
      const history = state.history.map((entry) =>
        entry.id === state.activeHistoryId
          ? { ...entry, events: [...entry.events, event].slice(-300) }
          : entry
      );
      return { history };
    });
  },
  appendHistoryEvent: (entryId, event) =>
    set((state) => {
      const history = state.history.map((entry) =>
        entry.id === entryId ? { ...entry, events: [...entry.events, event].slice(-300) } : entry
      );
      return { history };
    }),
  finishHistory: (entryId, patch) =>
    set((state) => {
      const history = state.history.map((entry) =>
        entry.id === entryId
          ? {
              ...entry,
              ...patch,
              finishedAt: new Date().toISOString(),
            }
          : entry
      );
      persistHistory(history);
      return {
        history,
        activeHistoryId: state.activeHistoryId === entryId ? null : state.activeHistoryId,
      };
    }),
  removeHistory: (entryId) =>
    set((state) => {
      const history = state.history.filter((entry) => entry.id !== entryId);
      persistHistory(history);
      return { history };
    }),
  clearHistory: () => {
    persistHistory([]);
    set({ history: [], activeHistoryId: null });
  },
}));
