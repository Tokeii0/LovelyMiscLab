import { create } from "zustand";

export type InspectorTab = "params" | "output" | "logs";

interface InspectorState {
  tab: InspectorTab;
  setTab: (t: InspectorTab) => void;
}

export const useInspectorStore = create<InspectorState>((set) => ({
  tab: "params",
  setTab: (tab) => set({ tab }),
}));
