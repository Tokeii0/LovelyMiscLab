import { create } from "zustand";

export type View = "canvas" | "galgame" | "modules" | "templates" | "runs" | "resources" | "settings";

/** One name per view, used by the left rail, the command palette and page titles. */
export const VIEW_LABEL: Record<View, string> = {
  canvas: "画布",
  galgame: "故事模式",
  modules: "模块库",
  templates: "流程模板",
  runs: "运行记录",
  resources: "资源库",
  settings: "设置",
};

/** Asked before leaving the current view; resolve false to stay (e.g. unsaved settings). */
type LeaveGuard = () => Promise<boolean>;

interface ViewState {
  view: View;
  leaveGuard: LeaveGuard | null;
  setView: (v: View) => void;
  setLeaveGuard: (g: LeaveGuard | null) => void;
}

/** Which left-rail section is active. */
export const useViewStore = create<ViewState>((set, get) => ({
  view: "canvas",
  leaveGuard: null,
  setView: (view) => {
    const { view: current, leaveGuard } = get();
    if (view === current) return;
    if (!leaveGuard) {
      set({ view });
      return;
    }
    void leaveGuard().then((ok) => {
      if (ok) set({ view, leaveGuard: null });
    });
  },
  setLeaveGuard: (leaveGuard) => set({ leaveGuard }),
}));
