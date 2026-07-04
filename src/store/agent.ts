import { create } from "zustand";

export type AgentStepKind =
  | "thinking"
  | "add"
  | "connect"
  | "param"
  | "run"
  | "result"
  | "error"
  | "done";

export interface AgentStep {
  kind: AgentStepKind;
  text: string;
  /** For result steps: whether the node ran ok. */
  ok?: boolean;
}

interface AgentState {
  running: boolean;
  job: string | null;
  steps: AgentStep[];
  notes: string;
  error: string;
  /** Set by the AI dialog; consumed by Canvas (which owns the ReactFlow handle). */
  pendingPrompt: string | null;

  launch: (prompt: string) => void;
  clearPending: () => void;
  start: () => void;
  setJob: (job: string) => void;
  pushStep: (s: AgentStep) => void;
  finish: (notes: string) => void;
  setError: (msg: string) => void;
  reset: () => void;
}

/** Drives the live AI agent: the step log the user watches + the run/cancel
 * state shown in AgentPanel. */
export const useAgentStore = create<AgentState>((set) => ({
  running: false,
  job: null,
  steps: [],
  notes: "",
  error: "",
  pendingPrompt: null,

  launch: (prompt) => set({ pendingPrompt: prompt }),
  clearPending: () => set({ pendingPrompt: null }),
  start: () => set({ running: true, steps: [], notes: "", error: "", job: null }),
  setJob: (job) => set({ job }),
  pushStep: (s) => set((st) => ({ steps: [...st.steps, s] })),
  finish: (notes) => set({ running: false, notes }),
  setError: (msg) => set({ running: false, error: msg }),
  reset: () => set({ running: false, job: null, steps: [], notes: "", error: "" }),
}));
