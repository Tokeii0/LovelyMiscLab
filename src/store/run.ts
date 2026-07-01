import { create } from "zustand";

export type RunMode = "idle" | "live" | "paused";

interface RunState {
  /** idle = not running; live = auto-run on changes; paused = frozen. */
  mode: RunMode;
  /** True while a run is in flight. */
  running: boolean;
  /** Duration of the last run, in ms. */
  elapsed: number;
  setMode: (m: RunMode) => void;
  setRunning: (r: boolean) => void;
  setElapsed: (ms: number) => void;
}

export const useRunStore = create<RunState>((set) => ({
  mode: "idle",
  running: false,
  elapsed: 0,
  setMode: (mode) => set({ mode }),
  setRunning: (running) => set({ running }),
  setElapsed: (elapsed) => set({ elapsed }),
}));
