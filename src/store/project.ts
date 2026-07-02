import { create } from "zustand";

/** The currently-open flow project: display name + on-disk path (null = unsaved). */
interface ProjectState {
  name: string;
  path: string | null;
  setName: (name: string) => void;
  setPath: (path: string | null) => void;
  reset: () => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  name: "未命名流程",
  path: null,
  setName: (name) => set({ name }),
  setPath: (path) => set({ path }),
  reset: () => set({ name: "未命名流程", path: null }),
}));
