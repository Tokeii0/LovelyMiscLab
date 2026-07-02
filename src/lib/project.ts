// New / Open / Save for the whole canvas flow. Saves the full frontend node
// state (labels, colors, positions, promoted params, disabled) — a superset of
// the executable SerializedGraph — so a project restores exactly as saved.

import { open, save } from "@tauri-apps/plugin-dialog";

import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { useGraphStore } from "@/store/graph";
import { useProjectStore } from "@/store/project";
import { useViewStore } from "@/store/view";

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

function nameFromPath(path: string): string {
  return path.split(/[\\/]/).pop()?.replace(/\.(lml|json)$/i, "") ?? "未命名流程";
}

function buildProject(): FlowProject {
  const g = useGraphStore.getState();
  return {
    version: 1,
    name: useProjectStore.getState().name,
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

function applyProject(json: string, path: string | null) {
  const project = JSON.parse(json) as FlowProject;
  if (!Array.isArray(project.nodes)) throw new Error("不是有效的流程文件");
  useGraphStore.getState().loadFlow(project.nodes, project.edges ?? []);
  useProjectStore.getState().setPath(path);
  useProjectStore.getState().setName(project.name || (path ? nameFromPath(path) : "未命名流程"));
  useViewStore.getState().setView("canvas");
}

/** Clear the canvas and start a fresh, unnamed project. */
export function newFlow() {
  if (useGraphStore.getState().nodes.length > 0 && !confirm("新建将清空当前画布，确定？")) return;
  useGraphStore.getState().clear();
  useProjectStore.getState().reset();
}

export async function saveFlow() {
  const project = buildProject();
  const json = JSON.stringify(project, null, 2);
  if (!inTauri) {
    downloadText(`${project.name || "流程"}.lml`, json);
    return;
  }
  let path = useProjectStore.getState().path;
  if (!path) {
    const chosen = await save({ filters: FILTERS, defaultPath: `${project.name || "流程"}.lml` });
    if (typeof chosen !== "string") return;
    path = chosen;
  }
  await api.saveProject(path, json);
  useProjectStore.getState().setPath(path);
  useProjectStore.getState().setName(nameFromPath(path));
}

export async function openFlow() {
  if (!inTauri) {
    openViaInput();
    return;
  }
  const chosen = await open({ filters: FILTERS, multiple: false, directory: false });
  if (typeof chosen !== "string") return;
  applyProject(await api.loadProject(chosen), chosen);
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
        console.error("open flow failed", e);
      }
    };
    r.readAsText(f);
  };
  input.click();
}
