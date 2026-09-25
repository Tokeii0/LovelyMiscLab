import { useCallback, useEffect, useMemo, useState } from "react";
import { Play, Plus, Trash2 } from "lucide-react";

import { api } from "@/lib/bindings";
import { categoryRank } from "@/lib/categories";
import { inTauri } from "@/lib/devMocks";
import { searchDescriptors } from "@/lib/nodeSearch";
import { cn } from "@/lib/utils";
import type { NodeDescriptor, PortSpec } from "@/lib/types";
import { confirmDialog } from "@/store/confirm";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { toast } from "@/store/toast";
import { useViewStore } from "@/store/view";
import { nodeIcon } from "@/flow/nodeIcons";
import { nodeSummary } from "@/flow/nodeDescriptions";
import { placeInView } from "@/flow/placement";
import { portColor, portTypeLabel } from "@/flow/portColors";

import { ModuleRunDialog } from "./ModuleRunDialog";

function Ports({ ports }: { ports: PortSpec[] }) {
  if (ports.length === 0) return <span className="text-muted-foreground/40">无</span>;
  return (
    <span className="flex gap-0.5">
      {ports.map((p) => (
        <span
          key={p.name}
          title={`${p.label}：${portTypeLabel(p.type)}`}
          className="h-1.5 w-1.5 rounded-full"
          style={{ background: portColor(p.type) }}
        />
      ))}
    </span>
  );
}

export function ModulesView() {
  const list = useDescriptorStore((s) => s.list);
  const setDescriptors = useDescriptorStore((s) => s.setDescriptors);
  const addNode = useGraphStore((s) => s.addNode);
  const setView = useViewStore((s) => s.setView);
  const [q, setQ] = useState("");
  const [cat, setCat] = useState("全部");
  const [runModule, setRunModule] = useState<NodeDescriptor | null>(null);

  // Which descriptors are the user's own (deletable) modules, and of which kind —
  // asked from the backend rather than guessed from the category label.
  const [userModules, setUserModules] = useState<Map<string, "composite" | "script">>(new Map());
  const refreshUserModules = useCallback(async () => {
    if (!inTauri) return;
    const [composites, scripts] = await Promise.all([api.listCompositeModules(), api.listScriptModules()]);
    setUserModules(
      new Map([
        ...composites.map((m) => [m.id, "composite"] as const),
        ...scripts.map((m) => [m.id, "script"] as const),
      ])
    );
  }, []);
  useEffect(() => {
    refreshUserModules().catch((e) => toast.error("读取自定义模块失败", { error: e }));
  }, [refreshUserModules, list]);

  const categories = useMemo(
    () => [
      "全部",
      ...Array.from(new Set(list.map((d) => d.category))).sort((a, b) => categoryRank(a) - categoryRank(b)),
    ],
    [list]
  );

  const filtered = useMemo(
    () => searchDescriptors(list, q).filter((d) => cat === "全部" || d.category === cat),
    [list, q, cat]
  );

  const addToCanvas = (d: NodeDescriptor) => {
    addNode(d, placeInView());
    setView("canvas");
  };

  const removeModule = async (d: NodeDescriptor) => {
    const ok = await confirmDialog({
      title: `删除模块「${d.displayName}」？`,
      message: "删除后无法恢复；画布上已使用它的节点将无法运行。",
      confirmText: "删除",
      danger: true,
    });
    if (!ok) return;
    try {
      if (inTauri) {
        if (userModules.get(d.id) === "script") await api.deleteScriptModule(d.id);
        else await api.deleteCompositeModule(d.id);
        setDescriptors(await api.listNodeDescriptors());
      } else {
        setDescriptors(useDescriptorStore.getState().list.filter((x) => x.id !== d.id));
      }
      toast.success(`已删除「${d.displayName}」`);
    } catch (e) {
      toast.error("删除模块失败", { error: e });
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border p-4">
        <h1 className="text-lg font-semibold">模块库</h1>
        <p className="text-xs text-muted-foreground">
          浏览全部模块 — 添加到画布编排流程，或直接单独调用执行。
        </p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="搜索模块…"
            className="w-64 rounded-md border border-input bg-background px-3 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <div className="flex flex-wrap gap-1">
            {categories.map((c) => (
              <button
                key={c}
                onClick={() => setCat(c)}
                className={cn(
                  "rounded-full px-3 py-1 text-xs transition-colors",
                  cat === c
                    ? "bg-primary text-primary-foreground"
                    : "bg-secondary text-muted-foreground hover:bg-accent hover:text-foreground"
                )}
              >
                {c}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-3 xl:grid-cols-4">
          {filtered.map((d) => {
            const Icon = nodeIcon(d.id, d.category);
            return (
              <div
                key={d.id}
                className="flex flex-col rounded-xl border border-border bg-card p-3 transition-all hover:border-primary hover:shadow-md"
              >
                <div className="flex items-start gap-2">
                  <span
                    className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg"
                    style={{ background: `${d.color}18`, color: d.color }}
                  >
                    <Icon className="h-5 w-5" />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{d.displayName}</div>
                    <div className="text-[10px] text-muted-foreground">{d.category}</div>
                  </div>
                </div>
                <p className="mt-2 line-clamp-2 h-8 text-xs text-muted-foreground">
                  {nodeSummary(d) || "—"}
                </p>
                <div className="mt-2 flex items-center gap-1 text-[10px] text-muted-foreground">
                  <Ports ports={d.inputs} />
                  <span className="opacity-50">→</span>
                  <Ports ports={d.outputs} />
                </div>
                <div className="mt-3 flex gap-1">
                  <button
                    onClick={() => addToCanvas(d)}
                    className="flex flex-1 items-center justify-center gap-1 rounded-md bg-secondary py-1 text-[11px] hover:bg-accent"
                  >
                    <Plus className="h-3 w-3" /> 加入画布
                  </button>
                  <button
                    onClick={() => setRunModule(d)}
                    className="flex flex-1 items-center justify-center gap-1 rounded-md border border-border py-1 text-[11px] hover:bg-accent"
                  >
                    <Play className="h-3 w-3" /> 单独调用
                  </button>
                  {(userModules.has(d.id) || (!inTauri && d.category === "自定义")) && (
                    <button
                      onClick={() => void removeModule(d)}
                      title="删除自定义模块"
                      className="flex items-center justify-center rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                    >
                      <Trash2 className="h-3 w-3" />
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {runModule && (
        <ModuleRunDialog descriptor={runModule} onClose={() => setRunModule(null)} />
      )}
    </div>
  );
}
