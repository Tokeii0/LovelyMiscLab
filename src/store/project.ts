import { create } from "zustand";

import { useGraphStore } from "@/store/graph";

export const DEFAULT_PROJECT_NAME = "未命名流程";

/** The currently-open flow project: display name + on-disk path (null = never saved). */
interface ProjectState {
  name: string;
  path: string | null;
  /** Graph `editRevision` at the last save/open. -1 = has changes no file holds. */
  savedRevision: number;
  setName: (name: string) => void;
  setPath: (path: string | null) => void;
  /** Record that the graph at `rev` now matches the file on disk. */
  markSaved: (rev: number) => void;
  markDirty: () => void;
  /** Forget the file: the canvas is now a new, unsaved document called `name`,
   * so Ctrl+S asks for a location instead of overwriting the old file. */
  detach: (name: string) => void;
  reset: () => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  name: DEFAULT_PROJECT_NAME,
  path: null,
  savedRevision: 0,
  setName: (name) => set({ name }),
  setPath: (path) => set({ path }),
  markSaved: (savedRevision) => set({ savedRevision }),
  markDirty: () => set({ savedRevision: -1 }),
  detach: (name) => set({ name, path: null, savedRevision: -1 }),
  reset: () => set({ name: DEFAULT_PROJECT_NAME, path: null, savedRevision: -1 }),
}));

/** True when the canvas has edits that aren't in any saved file. An empty,
 * never-saved canvas has nothing to lose and never counts as dirty. */
export function isProjectDirty(): boolean {
  const { savedRevision, path } = useProjectStore.getState();
  const g = useGraphStore.getState();
  if (!path && g.nodes.length === 0) return false;
  return savedRevision !== g.editRevision;
}

export function useProjectDirty(): boolean {
  const savedRevision = useProjectStore((s) => s.savedRevision);
  const path = useProjectStore((s) => s.path);
  const editRevision = useGraphStore((s) => s.editRevision);
  const empty = useGraphStore((s) => s.nodes.length === 0);
  if (!path && empty) return false;
  return savedRevision !== editRevision;
}
