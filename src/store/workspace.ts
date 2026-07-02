import { create } from "zustand";

export type ResourceKind = "sample" | "dictionary" | "script" | "artifact" | "note";

export interface ResourceItem {
  id: string;
  kind: ResourceKind;
  name: string;
  path: string;
  note: string;
  tags: string[];
  addedAt: string;
}

export interface RecentProject {
  path: string;
  name: string;
  openedAt: string;
}

const STORAGE_KEY = "misclab-workspace-v1";
const MAX_RECENTS = 12;

interface PersistedWorkspace {
  resources: ResourceItem[];
  recentProjects: RecentProject[];
}

function loadWorkspace(): PersistedWorkspace {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { resources: [], recentProjects: [] };
    const parsed = JSON.parse(raw) as PersistedWorkspace;
    return {
      resources: Array.isArray(parsed.resources) ? parsed.resources : [],
      recentProjects: Array.isArray(parsed.recentProjects) ? parsed.recentProjects : [],
    };
  } catch {
    return { resources: [], recentProjects: [] };
  }
}

function persist(state: PersistedWorkspace) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    /* ignore storage failures */
  }
}

function basename(path: string) {
  return path.split(/[\\/]/).pop() || path || "未命名资源";
}

interface WorkspaceState extends PersistedWorkspace {
  addResource: (resource: Omit<ResourceItem, "id" | "addedAt">) => void;
  updateResource: (id: string, patch: Partial<Omit<ResourceItem, "id">>) => void;
  removeResource: (id: string) => void;
  clearResources: () => void;
  addRecentProject: (path: string, name: string) => void;
  removeRecentProject: (path: string) => void;
  clearRecentProjects: () => void;
}

const initial = loadWorkspace();

export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  ...initial,
  addResource: (resource) =>
    set((state) => {
      const item: ResourceItem = {
        ...resource,
        id: `res_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
        name: resource.name.trim() || basename(resource.path),
        addedAt: new Date().toISOString(),
      };
      const resources = [item, ...state.resources];
      persist({ resources, recentProjects: state.recentProjects });
      return { resources };
    }),
  updateResource: (id, patch) =>
    set((state) => {
      const resources = state.resources.map((item) =>
        item.id === id ? { ...item, ...patch } : item
      );
      persist({ resources, recentProjects: state.recentProjects });
      return { resources };
    }),
  removeResource: (id) =>
    set((state) => {
      const resources = state.resources.filter((item) => item.id !== id);
      persist({ resources, recentProjects: state.recentProjects });
      return { resources };
    }),
  clearResources: () =>
    set((state) => {
      persist({ resources: [], recentProjects: state.recentProjects });
      return { resources: [] };
    }),
  addRecentProject: (path, name) =>
    set((state) => {
      const openedAt = new Date().toISOString();
      const recentProjects = [
        { path, name: name.trim() || basename(path), openedAt },
        ...state.recentProjects.filter((p) => p.path !== path),
      ].slice(0, MAX_RECENTS);
      persist({ resources: state.resources, recentProjects });
      return { recentProjects };
    }),
  removeRecentProject: (path) =>
    set((state) => {
      const recentProjects = state.recentProjects.filter((p) => p.path !== path);
      persist({ resources: state.resources, recentProjects });
      return { recentProjects };
    }),
  clearRecentProjects: () =>
    set((state) => {
      persist({ resources: state.resources, recentProjects: [] });
      return { recentProjects: [] };
    }),
}));

export function addRecentProject(path: string, name: string) {
  useWorkspaceStore.getState().addRecentProject(path, name);
}

export function currentWorkspaceSnapshot() {
  const { resources, recentProjects } = getWorkspaceState();
  return { resources, recentProjects };
}

function getWorkspaceState() {
  return useWorkspaceStore.getState();
}
