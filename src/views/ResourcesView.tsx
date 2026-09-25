import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Archive,
  BookOpen,
  Database,
  FileCode2,
  FolderOpen,
  HardDrive,
  Package,
  Plus,
  RotateCcw,
  Search,
  StickyNote,
  Trash2,
  type LucideIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Empty } from "@/components/ui/empty";
import { placeInView } from "@/flow/placement";
import { inTauri } from "@/lib/devMocks";
import { openFlowPath, readAutoDraft, restoreAutoDraft } from "@/lib/project";
import { confirmDialog } from "@/store/confirm";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useViewStore, VIEW_LABEL } from "@/store/view";
import { useWorkspaceStore, type ResourceItem, type ResourceKind } from "@/store/workspace";

export function ResourcesView() {
  const resources = useWorkspaceStore((s) => s.resources);
  const recentProjects = useWorkspaceStore((s) => s.recentProjects);
  const addResource = useWorkspaceStore((s) => s.addResource);
  const removeResource = useWorkspaceStore((s) => s.removeResource);
  const clearResources = useWorkspaceStore((s) => s.clearResources);
  const removeRecentProject = useWorkspaceStore((s) => s.removeRecentProject);
  const fileImport = useDescriptorStore((s) => s.byId.file_import);
  const addNode = useGraphStore((s) => s.addNode);
  const setParam = useGraphStore((s) => s.setParam);
  const setView = useViewStore((s) => s.setView);
  const [kind, setKind] = useState<ResourceKind>("sample");
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [tags, setTags] = useState("");
  const [note, setNote] = useState("");
  const [query, setQuery] = useState("");
  const [draft, setDraft] = useState(readAutoDraft());

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return resources;
    return resources.filter(
      (r) =>
        r.name.toLowerCase().includes(q) ||
        r.path.toLowerCase().includes(q) ||
        r.note.toLowerCase().includes(q) ||
        r.tags.some((t) => t.toLowerCase().includes(q))
    );
  }, [query, resources]);

  const pickPath = async () => {
    if (!inTauri) return;
    const selected = await open({ multiple: false, directory: false });
    if (typeof selected === "string") {
      setPath(selected);
      if (!name.trim()) setName(selected.split(/[\\/]/).pop() ?? selected);
    }
  };

  const submit = () => {
    if (!path.trim() && kind !== "note") return;
    addResource({
      kind,
      name,
      path: path.trim(),
      note: note.trim(),
      tags: tags
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean),
    });
    setName("");
    setPath("");
    setTags("");
    setNote("");
  };

  const addToCanvas = (resource: ResourceItem) => {
    if (!fileImport || !resource.path) return;
    setView("canvas");
    useGraphStore.getState().transact(() => {
      const id = addNode(fileImport, placeInView());
      setParam(id, "path", resource.path);
    });
  };

  const restoreDraft = async () => {
    if (await restoreAutoDraft()) setDraft(readAutoDraft());
  };

  return (
    <div className="grid h-full min-h-0 grid-cols-[320px_1fr]">
      <aside className="min-h-0 overflow-y-auto border-r border-border bg-card p-4">
        <div className="mb-4">
          <h1 className="text-lg font-semibold">{VIEW_LABEL.resources}</h1>
          <p className="mt-1 text-xs text-muted-foreground">
            管理样本、字典、脚本路径、最近流程和自动保存草稿。
          </p>
        </div>

        <section className="mb-4 rounded-lg border border-border bg-background p-3">
          <div className="mb-2 flex items-center gap-2 text-sm font-semibold">
            <RotateCcw className="h-4 w-4 text-primary" />
            自动保存草稿
          </div>
          {draft ? (
            <>
              <div className="text-xs text-muted-foreground">
                {draft.project.name} · {new Date(draft.savedAt).toLocaleString()}
              </div>
              <Button className="mt-3 w-full" size="sm" onClick={() => void restoreDraft()}>
                恢复草稿
              </Button>
            </>
          ) : (
            <div className="text-xs text-muted-foreground">当前没有可恢复草稿。</div>
          )}
        </section>

        <section>
          <div className="mb-2 flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-semibold">
              <HardDrive className="h-4 w-4 text-primary" />
              最近项目
            </div>
          </div>
          {recentProjects.length === 0 ? (
            <div className="rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">
              保存或打开流程后会出现在这里。
            </div>
          ) : (
            <div className="space-y-2">
              {recentProjects.map((project) => (
                <div key={project.path} className="rounded-md border border-border bg-background p-2">
                  <div className="truncate text-xs font-medium">{project.name}</div>
                  <div className="mt-0.5 truncate text-[10px] text-muted-foreground" title={project.path}>
                    {project.path}
                  </div>
                  <div className="mt-2 flex gap-1">
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-7 flex-1"
                      onClick={() => void openFlowPath(project.path)}
                    >
                      打开
                    </Button>
                    <button
                      onClick={() => removeRecentProject(project.path)}
                      title="移除"
                      className="rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </aside>

      <main className="flex min-h-0 flex-col">
        <div className="border-b border-border p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold">资源库</h2>
              <p className="text-xs text-muted-foreground">
                样本和字典保留路径索引；样本可一键生成文件导入节点。
              </p>
            </div>
            {resources.length > 0 && (
              <Button
                variant="outline"
                size="sm"
                disabled={resources.length === 0}
                onClick={async () => {
                  const ok = await confirmDialog({
                    title: "清空资源库？",
                    message: `将移除 ${resources.length} 条资源记录（不会删除磁盘上的文件）。`,
                    confirmText: "清空",
                    danger: true,
                  });
                  if (ok) clearResources();
                }}
              >
                <Trash2 className="h-3.5 w-3.5" />
                清空资源
              </Button>
            )}
          </div>

          <div className="mt-3 grid grid-cols-[120px_1fr_140px] gap-2">
            <select
              value={kind}
              onChange={(e) => setKind(e.target.value as ResourceKind)}
              className="rounded-md border border-input bg-background px-2 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
            >
              {RESOURCE_KINDS.map((k) => (
                <option key={k.kind} value={k.kind}>
                  {k.label}
                </option>
              ))}
            </select>
            <input
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder={kind === "note" ? "可留空，作为纯备注资源" : "资源路径"}
              className="rounded-md border border-input bg-background px-2 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
            />
            <Button variant="outline" size="sm" onClick={() => void pickPath()} disabled={!inTauri}>
              <FolderOpen className="h-3.5 w-3.5" />
              选择
            </Button>
          </div>
          <div className="mt-2 grid grid-cols-[220px_1fr_120px] gap-2">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="资源名称"
              className="rounded-md border border-input bg-background px-2 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
            />
            <input
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="标签，用逗号分隔"
              className="rounded-md border border-input bg-background px-2 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
            />
            <Button size="sm" onClick={submit} disabled={!path.trim() && kind !== "note"}>
              <Plus className="h-3.5 w-3.5" />
              添加
            </Button>
          </div>
          <textarea
            value={note}
            onChange={(e) => setNote(e.target.value)}
            rows={2}
            placeholder="备注：密码、用途、来源或处理线索"
            className="mt-2 w-full resize-none rounded-md border border-input bg-background px-2 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <div className="mt-3 flex items-center gap-2 rounded-md border border-border bg-background px-2 py-1.5">
            <Search className="h-3.5 w-3.5 text-muted-foreground" />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="搜索名称、路径、标签或备注"
              className="flex-1 bg-transparent text-xs focus:outline-none"
            />
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          {filtered.length === 0 ? (
            <Empty
              icon={Package}
              title="暂无匹配资源"
              hint="添加样本、字典、脚本路径或备注后，可以在这里统一检索。"
            />
          ) : (
            <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
              {filtered.map((resource) => {
                const meta = RESOURCE_META[resource.kind];
                const Icon = meta.icon;
                return (
                  <div key={resource.id} className="rounded-lg border border-border bg-card p-3">
                    <div className="flex items-start gap-2">
                      <span
                        className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md"
                        style={{ background: `${meta.color}18`, color: meta.color }}
                      >
                        <Icon className="h-4 w-4" />
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <div className="truncate text-sm font-medium">{resource.name}</div>
                          <span className="rounded bg-secondary px-1.5 py-0.5 text-[10px] text-muted-foreground">
                            {meta.label}
                          </span>
                        </div>
                        {resource.path && (
                          <div className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground" title={resource.path}>
                            {resource.path}
                          </div>
                        )}
                      </div>
                      <button
                        onClick={() => removeResource(resource.id)}
                        title="删除资源"
                        className="rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    </div>
                    {resource.note && (
                      <div className="mt-2 line-clamp-2 text-xs text-muted-foreground">{resource.note}</div>
                    )}
                    {resource.tags.length > 0 && (
                      <div className="mt-2 flex flex-wrap gap-1">
                        {resource.tags.map((tag) => (
                          <span key={tag} className="rounded bg-secondary px-1.5 py-0.5 text-[10px] text-muted-foreground">
                            {tag}
                          </span>
                        ))}
                      </div>
                    )}
                    <div className="mt-3 flex items-center justify-between gap-2">
                      <span className="text-[10px] text-muted-foreground">
                        {new Date(resource.addedAt).toLocaleString()}
                      </span>
                      {resource.path && (
                        <Button variant="outline" size="sm" onClick={() => addToCanvas(resource)}>
                          <FileCode2 className="h-3.5 w-3.5" />
                          加入画布
                        </Button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </main>
    </div>
  );
}

const RESOURCE_KINDS: { kind: ResourceKind; label: string }[] = [
  { kind: "sample", label: "样本" },
  { kind: "dictionary", label: "字典" },
  { kind: "script", label: "脚本" },
  { kind: "artifact", label: "工件" },
  { kind: "note", label: "备注" },
];

const RESOURCE_META: Record<ResourceKind, { label: string; color: string; icon: LucideIcon }> = {
  sample: { label: "样本", color: "#2563eb", icon: Database },
  dictionary: { label: "字典", color: "#16a34a", icon: BookOpen },
  script: { label: "脚本", color: "#9333ea", icon: FileCode2 },
  artifact: { label: "工件", color: "#ea580c", icon: Archive },
  note: { label: "备注", color: "#64748b", icon: StickyNote },
};
