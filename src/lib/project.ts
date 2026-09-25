// New / Open / Save for the whole canvas flow. Saves the full frontend node
// state (labels, colors, positions, promoted params, disabled) — a superset of
// the executable SerializedGraph — so a project restores exactly as saved.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";

import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { choose } from "@/store/confirm";
import { breakHistoryMerge, useGraphStore } from "@/store/graph";
import { DEFAULT_PROJECT_NAME, isProjectDirty, useProjectStore } from "@/store/project";
import { toast } from "@/store/toast";
import { useViewStore } from "@/store/view";
import { addRecentProject, useWorkspaceStore } from "@/store/workspace";

export interface SavedNode {
  id: string;
  descriptorId: string;
  label: string;
  color: string;
  params: Record<string, unknown>;
  inputParams: string[];
  disabled: boolean;
  position: { x: number; y: number };
}
export interface SavedEdge {
  id: string;
  source: string;
  sourceHandle: string | null;
  target: string;
  targetHandle: string | null;
  type?: string;
}
export interface FlowProject {
  version: 1;
  name: string;
  nodes: SavedNode[];
  edges: SavedEdge[];
}

const FILTERS = [{ name: "LovelyMiscLab 流程", extensions: ["lml", "json"] }];
export const AUTOSAVE_KEY = "misclab-autosave-v1";

function nameFromPath(path: string): string {
  return path.split(/[\\/]/).pop()?.replace(/\.(lml|json)$/i, "") ?? DEFAULT_PROJECT_NAME;
}

/** The current canvas as `SavedNode[]`/`SavedEdge[]` (all nodes, incl. disabled).
 *  Shared by project save and the MCP canvas sync so both agree on the shape. */
export function graphToSaved(): { nodes: SavedNode[]; edges: SavedEdge[] } {
  const g = useGraphStore.getState();
  return {
    nodes: g.nodes.map((n) => ({
      id: n.id,
      descriptorId: n.data.descriptorId,
      label: n.data.label,
      color: n.data.color,
      params: n.data.params,
      inputParams: n.data.inputParams ?? [],
      disabled: n.data.disabled ?? false,
      position: { x: n.position.x, y: n.position.y },
    })),
    edges: g.edges.map((e) => ({
      id: e.id,
      source: e.source,
      sourceHandle: e.sourceHandle ?? null,
      target: e.target,
      targetHandle: e.targetHandle ?? null,
      type: e.type,
    })),
  };
}

export function buildProject(): FlowProject {
  return {
    version: 1,
    name: useProjectStore.getState().name,
    ...graphToSaved(),
  };
}

/** Load a project file's JSON into the canvas (wipes undo — it's a different document). */
export function applyProject(json: string, path: string | null) {
  const project = JSON.parse(json) as FlowProject;
  if (!Array.isArray(project.nodes)) throw new Error("不是有效的流程文件");
  useGraphStore.getState().loadFlow(project.nodes, project.edges ?? []);
  const ps = useProjectStore.getState();
  ps.setPath(path);
  ps.setName(project.name || (path ? nameFromPath(path) : DEFAULT_PROJECT_NAME));
  ps.markSaved(useGraphStore.getState().editRevision);
  if (path) addRecentProject(path, project.name || nameFromPath(path));
  useViewStore.getState().setView("canvas");
}

type UnsavedChoice = "clean" | "saved" | "discarded";

/** Ask what to do with unsaved edits before they'd be replaced. null = cancel. */
async function askUnsaved(action: string): Promise<UnsavedChoice | null> {
  if (!isProjectDirty()) return "clean";
  const pick = await choose({
    title: "当前流程有未保存的修改",
    message: `${action}前，要先保存「${useProjectStore.getState().name}」吗？`,
    options: [
      { value: "discard" as const, label: "不保存", variant: "outline" },
      { value: "save" as const, label: "保存" },
    ],
  });
  if (pick === "save") return (await saveFlow()) ? "saved" : null;
  return pick === "discard" ? "discarded" : null;
}

/** Resolves true when it's OK to replace the canvas (saved, discarded, or nothing to lose). */
export async function guardUnsaved(action = "继续"): Promise<boolean> {
  return (await askUnsaved(action)) !== null;
}

/** Clear the canvas and start a fresh, unnamed project. */
export async function newFlow() {
  if (!(await guardUnsaved("新建"))) return;
  useGraphStore.getState().loadFlow([], []);
  useProjectStore.getState().reset();
  useProjectStore.getState().markSaved(useGraphStore.getState().editRevision);
  useViewStore.getState().setView("canvas");
}

/** Save to the current file (or ask where). `as` always asks. Resolves true when written. */
export async function saveFlow(opts: { as?: boolean } = {}): Promise<boolean> {
  const ps = useProjectStore.getState();
  try {
    if (!inTauri) {
      const project = buildProject();
      downloadText(`${project.name || "流程"}.lml`, JSON.stringify(project, null, 2));
      ps.markSaved(useGraphStore.getState().editRevision);
      toast.success("已下载流程文件");
      return true;
    }
    let path = opts.as ? null : ps.path;
    if (!path) {
      const chosen = await save({ filters: FILTERS, defaultPath: `${ps.name || "流程"}.lml` });
      if (typeof chosen !== "string") return false;
      path = chosen;
    }
    // An untitled flow takes the file's name; a name the user picked is kept.
    if (ps.name === DEFAULT_PROJECT_NAME || !ps.name.trim()) ps.setName(nameFromPath(path));
    const rev = useGraphStore.getState().editRevision;
    await api.saveProject(path, JSON.stringify(buildProject(), null, 2));
    ps.setPath(path);
    ps.markSaved(rev);
    breakHistoryMerge();
    addRecentProject(path, useProjectStore.getState().name);
    toast.success(`已保存「${useProjectStore.getState().name}」`, { detail: path });
    return true;
  } catch (e) {
    toast.error("保存失败", { error: e });
    return false;
  }
}

export function saveFlowAs(): Promise<boolean> {
  return saveFlow({ as: true });
}

export async function openFlow() {
  if (!(await guardUnsaved("打开其他流程"))) return;
  if (!inTauri) {
    openViaInput();
    return;
  }
  const chosen = await open({ filters: FILTERS, multiple: false, directory: false });
  if (typeof chosen !== "string") return;
  await loadPath(chosen);
}

export async function openFlowPath(path: string) {
  if (!inTauri) return;
  if (!(await guardUnsaved("打开其他流程"))) return;
  await loadPath(path);
}

async function loadPath(path: string) {
  try {
    applyProject(await api.loadProject(path), path);
  } catch (e) {
    toast.error("无法打开流程", {
      error: e,
      actions: [
        {
          label: "从最近列表移除",
          run: () => useWorkspaceStore.getState().removeRecentProject(path),
        },
      ],
    });
  }
}

/** Rename the flow (the name is saved into the file, so this is an unsaved change). */
export function renameFlow(name: string) {
  const next = name.trim();
  const ps = useProjectStore.getState();
  if (!next || next === ps.name) return;
  ps.setName(next);
  ps.markDirty();
}

// ---- crash/close recovery draft ----

export interface AutoSaveDraft {
  savedAt: string;
  project: FlowProject;
  /** File the draft belongs to (null = never saved). */
  path?: string | null;
  /** Whether the draft holds edits the file doesn't. Missing in old drafts = assume yes. */
  dirty?: boolean;
}

/** While a startup "restore draft?" offer is open, don't overwrite the draft. */
let draftHeld = false;
let draftOffered = false;

export function saveAutoDraft() {
  if (draftHeld) return;
  try {
    const draft: AutoSaveDraft = {
      savedAt: new Date().toISOString(),
      project: buildProject(),
      path: useProjectStore.getState().path,
      dirty: isProjectDirty(),
    };
    localStorage.setItem(AUTOSAVE_KEY, JSON.stringify(draft));
  } catch {
    /* ignore storage failures */
  }
}

export function readAutoDraft(): AutoSaveDraft | null {
  try {
    const raw = localStorage.getItem(AUTOSAVE_KEY);
    if (!raw) return null;
    const draft = JSON.parse(raw) as AutoSaveDraft;
    return draft?.project && Array.isArray(draft.project.nodes) ? draft : null;
  } catch {
    return null;
  }
}

export function clearAutoDraft() {
  try {
    localStorage.removeItem(AUTOSAVE_KEY);
  } catch {
    /* ignore */
  }
}

/** Load the recovery draft (asking about unsaved work first). Resolves true when applied. */
export async function restoreAutoDraft(): Promise<boolean> {
  const draft = readAutoDraft();
  if (!draft) return false;
  if (!(await guardUnsaved("恢复草稿"))) return false;
  try {
    applyProject(JSON.stringify(draft.project), draft.path ?? null);
  } catch (e) {
    toast.error("草稿无法恢复", { error: e });
    return false;
  }
  if (draft.dirty !== false) useProjectStore.getState().markDirty();
  return true;
}

/** On startup: if the last session ended with unsaved edits, offer to bring them back. */
export function offerDraftRestore() {
  if (draftOffered) return;
  draftOffered = true;
  const draft = readAutoDraft();
  if (!draft || draft.dirty === false || draft.project.nodes.length === 0) return;
  draftHeld = true;
  const release = () => {
    draftHeld = false;
  };
  toast.info("发现上次未保存的流程", {
    detail: `${draft.project.name} · ${new Date(draft.savedAt).toLocaleString()}`,
    duration: 0,
    onDismiss: release,
    actions: [
      {
        label: "恢复",
        run: () => {
          release();
          void restoreAutoDraft();
        },
      },
      { label: "忽略", run: release },
    ],
  });
}

/** Ask before the window closes with unsaved edits. Returns a cleanup fn. */
export function installCloseGuard(): () => void {
  if (!inTauri) {
    const handler = (e: BeforeUnloadEvent) => {
      if (isProjectDirty()) e.preventDefault();
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }
  const unlisten = getCurrentWindow().onCloseRequested(async (event) => {
    const choice = await askUnsaved("关闭窗口");
    if (choice === null) event.preventDefault();
    // Explicitly discarded: don't offer those edits back on next launch.
    else if (choice === "discarded") clearAutoDraft();
  });
  return () => {
    void unlisten.then((f) => f());
  };
}

// ---- browser fallbacks (no Tauri fs/dialog) ----
function downloadText(filename: string, content: string) {
  const url = URL.createObjectURL(new Blob([content], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
function openViaInput() {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".lml,.json,application/json";
  input.onchange = () => {
    const f = input.files?.[0];
    if (!f) return;
    const r = new FileReader();
    r.onload = () => {
      try {
        applyProject(String(r.result), null);
      } catch (e) {
        toast.error("无法打开流程", { error: e });
      }
    };
    r.onerror = () => toast.error("读取文件失败", { error: r.error });
    r.readAsText(f);
  };
  input.click();
}
