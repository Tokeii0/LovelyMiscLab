import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Archive,
  BookOpen,
  Clock3,
  Database,
  FileCode2,
  FolderOpen,
  HardDrive,
  History,
  Package,
  Plus,
  RotateCcw,
  Search,
  StickyNote,
  Trash2,
  type LucideIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { inTauri } from "@/lib/devMocks";
import { openFlowPath, readAutoDraft, restoreAutoDraft } from "@/lib/project";
import { cn } from "@/lib/utils";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useRunStore, type RunHistoryEntry, type RunStatus } from "@/store/run";
import { useViewStore } from "@/store/view";
import {
  useWorkspaceStore,
  type ResourceItem,
  type ResourceKind,
} from "@/store/workspace";

function Empty({
  icon: Icon,
  title,
  hint,
}: {
  icon: LucideIcon;
  title: string;
  hint: string;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
      <span className="flex h-14 w-14 items-center justify-center rounded-lg bg-secondary text-muted-foreground">
        <Icon className="h-7 w-7" />
      </span>
      <div className="text-sm font-medium text-foreground">{title}</div>
      <div className="max-w-xs text-xs text-muted-foreground">{hint}</div>
    </div>
  );
}

function fmtTime(iso: string) {
  return new Date(iso).toLocaleString();
}

function statusTone(status: RunStatus) {
  if (status === "success") return "bg-green-500/15 text-green-600";
  if (status === "error") return "bg-destructive/10 text-destructive";
  if (status === "cancelled") return "bg-amber-500/15 text-amber-600";
  return "bg-blue-500/15 text-blue-600";
}

function StatusBadge({ status }: { status: RunStatus }) {
  const text = {
    running: "运行中",
    success: "成功",
    error: "失败",
    cancelled: "已取消",
  }[status];
  return (
    <span className={cn("rounded px-1.5 py-0.5 text-[10px] font-medium", statusTone(status))}>
      {text}
    </span>
  );
}

function RunListItem({
  entry,
  active,
  onClick,
}: {
  entry: RunHistoryEntry;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "w-full rounded-md border p-3 text-left transition-colors",
        active
          ? "border-primary bg-primary/5"
          : "border-border bg-card hover:border-primary/60 hover:bg-accent/40"
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0 truncate text-sm font-medium">{entry.title}</div>
        <StatusBadge status={entry.status} />
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-2 text-[10px] text-muted-foreground">
        <span>{entry.projectName}</span>
        <span>{entry.nodeCount} 节点</span>
        <span>{(entry.elapsed / 1000).toFixed(2)}s</span>
      </div>
      <div className="mt-1 text-[10px] text-muted-foreground">{fmtTime(entry.startedAt)}</div>
    </button>
  );
}

function RunDetail({ entry }: { entry: RunHistoryEntry }) {
  const failed = entry.nodes.filter((n) => n.status === "error").length;
  const done = entry.nodes.filter((n) => n.status === "done").length;
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-b border-border px-4 py-3">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="truncate text-base font-semibold">{entry.title}</div>
            <div className="mt-1 text-xs text-muted-foreground">
              {fmtTime(entry.startedAt)} · {entry.projectName}
            </div>
          </div>
          <StatusBadge status={entry.status} />
        </div>
        <div className="mt-3 grid grid-cols-4 gap-2">
          {[
            ["节点", entry.nodeCount],
            ["连接", entry.edgeCount],
            ["成功", done],
            ["失败", failed],
          ].map(([label, value]) => (
            <div key={label} className="rounded-md border border-border bg-background px-3 py-2">
              <div className="text-[10px] text-muted-foreground">{label}</div>
              <div className="text-sm font-semibold">{value}</div>
            </div>
          ))}
        </div>
        {entry.error && (
          <div className="mt-3 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive">
            {entry.error}
          </div>
        )}
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-[260px_1fr]">
        <div className="min-h-0 overflow-y-auto border-r border-border p-3">
          <div className="mb-2 text-xs font-semibold text-muted-foreground">节点结果</div>
          <div className="space-y-1.5">
            {entry.nodes.map((node) => (
              <div key={node.id} className="rounded-md border border-border bg-card px-2 py-1.5">
                <div className="flex items-center justify-between gap-2">
                  <span className="min-w-0 truncate text-xs font-medium">{node.label}</span>
                  <span className="text-[10px] text-muted-foreground">{node.status}</span>
                </div>
                {node.error && (
                  <div className="mt-1 line-clamp-2 text-[10px] text-destructive">{node.error}</div>
                )}
              </div>
            ))}
          </div>
        </div>
        <div className="min-h-0 overflow-y-auto p-3">
          <div className="mb-2 flex items-center gap-1 text-xs font-semibold text-muted-foreground">
            <Clock3 className="h-3.5 w-3.5" />
            事件流
          </div>
          {entry.events.length === 0 ? (
            <div className="text-xs text-muted-foreground">暂无事件</div>
          ) : (
            <div className="space-y-1 font-mono text-[11px]">
              {entry.events.map((event, i) => (
                <div key={`${event.time}-${i}`} className="flex gap-2 rounded px-2 py-1 hover:bg-accent/40">
                  <span className="w-20 shrink-0 text-muted-foreground">{event.time}</span>
                  <span
                    className={cn(
                      "w-14 shrink-0",
                      event.level === "error"
                        ? "text-destructive"
                        : event.level === "success"
                          ? "text-green-600"
                          : event.level === "warn"
                            ? "text-amber-600"
                            : "text-muted-foreground"
                    )}
                  >
                    {event.level}
                  </span>
                  {event.node && (
                    <span className="max-w-32 shrink-0 truncate text-muted-foreground">
                      {event.node}
                    </span>
                  )}
                  <span className="min-w-0 break-all">{event.message}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export function RunsView() {
  const history = useRunStore((s) => s.history);
  const clearHistory = useRunStore((s) => s.clearHistory);
  const removeHistory = useRunStore((s) => s.removeHistory);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = useMemo(
    () => history.find((entry) => entry.id === selectedId) ?? history[0],
    [history, selectedId]
  );

  if (history.length === 0) {
    return (
      <div className="h-full">
        <Empty
          icon={History}
          title="暂无运行记录"
          hint="运行整图、单节点或调试子图后，这里会保留耗时、状态、节点结果和事件流。"
        />
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div>
          <h1 className="text-lg font-semibold">运行记录</h1>
          <p className="text-xs text-muted-foreground">回看每次执行的路径、耗时和失败点。</p>
        </div>
        <Button variant="outline" size="sm" onClick={clearHistory}>
          <Trash2 className="h-3.5 w-3.5" />
          清空记录
        </Button>
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-[320px_1fr]">
        <aside className="min-h-0 overflow-y-auto border-r border-border p-3">
          <div className="space-y-2">
            {history.map((entry) => (
              <div key={entry.id} className="group relative">
                <RunListItem
                  entry={entry}
                  active={selected?.id === entry.id}
                  onClick={() => setSelectedId(entry.id)}
                />
                <button
                  onClick={() => removeHistory(entry.id)}
                  title="删除记录"
                  className="absolute right-2 top-9 hidden rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive group-hover:block"
                >
                  <Trash2 className="h-3 w-3" />
                </button>
              </div>
            ))}
          </div>
        </aside>
        {selected ? (
          <RunDetail entry={selected} />
        ) : (
          <Empty icon={RotateCcw} title="选择一条记录" hint="从左侧选择运行记录查看详情。" />
        )}
      </div>
    </div>
  );
}

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
    const id = addNode(fileImport, { x: 240 + Math.random() * 120, y: 160 + Math.random() * 120 });
    setParam(id, "path", resource.path);
    setView("canvas");
  };

  const restoreDraft = () => {
    if (restoreAutoDraft()) setDraft(readAutoDraft());
  };

  return (
    <div className="grid h-full min-h-0 grid-cols-[320px_1fr]">
      <aside className="min-h-0 overflow-y-auto border-r border-border bg-card p-4">
        <div className="mb-4">
          <h1 className="text-lg font-semibold">资源与工作区</h1>
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
              <Button className="mt-3 w-full" size="sm" onClick={restoreDraft}>
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
              <Button variant="outline" size="sm" onClick={clearResources}>
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
