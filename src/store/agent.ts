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
  /** The one-line "巧思" the agent gave for this step, shown dimmer. */
  detail?: string;
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
  /** Optional long input (e.g. ciphertext) the app injects into the source node
   * directly, so the model never has to echo it. */
  pendingData: string | null;

  launch: (prompt: string, data?: string | null) => void;
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
  pendingData: null,

  launch: (prompt, data) => set({ pendingPrompt: prompt, pendingData: data ?? null }),
  clearPending: () => set({ pendingPrompt: null, pendingData: null }),
  start: () => set({ running: true, steps: [], notes: "", error: "", job: null }),
  setJob: (job) => set({ job }),
  pushStep: (s) => set((st) => ({ steps: [...st.steps, s] })),
  finish: (notes) => set({ running: false, notes }),
  setError: (msg) => set({ running: false, error: msg }),
  reset: () => set({ running: false, job: null, steps: [], notes: "", error: "" }),
}));
