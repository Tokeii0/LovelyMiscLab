import { create } from "zustand";

export type View = "canvas" | "modules" | "templates" | "runs" | "resources" | "settings";

interface ViewState {
  view: View;
  setView: (v: View) => void;
}

/** Which left-rail section is active. */
export const useViewStore = create<ViewState>((set) => ({
  view: "canvas",
  setView: (view) => set({ view }),
}));
