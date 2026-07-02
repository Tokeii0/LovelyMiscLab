import { useMemo, useState } from "react";
import { BookOpen, Cable, Keyboard, Search, Workflow, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useDescriptorStore } from "@/store/descriptors";
import { useHelpStore } from "@/store/help";

type Tab = "nodes" | "shortcuts" | "recipes";

export function HelpDialog() {
  const open = useHelpStore((s) => s.open);
  const setOpen = useHelpStore((s) => s.setOpen);
  const initialId = useHelpStore((s) => s.descriptorId);
  const descriptors = useDescriptorStore((s) => s.list);
  const byId = useDescriptorStore((s) => s.byId);
  const [tab, setTab] = useState<Tab>("nodes");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(initialId);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return descriptors;
    return descriptors.filter(
      (d) =>
        d.displayName.toLowerCase().includes(q) ||
        d.id.toLowerCase().includes(q) ||
        d.category.toLowerCase().includes(q) ||
        (d.description ?? "").toLowerCase().includes(q)
    );
  }, [descriptors, query]);

  if (!open) return null;

  const selected = (selectedId && byId[selectedId]) || filtered[0];
  const close = () => setOpen(false);

  return (
    <div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/45 p-4" onClick={close}>
      <div
        className="flex h-[78vh] w-[960px] max-w-[96vw] flex-col overflow-hidden rounded-lg border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border px-4 py-3">
          <span className="flex h-8 w-8 items-center justify-center rounded-md bg-primary/10 text-primary">
            <BookOpen className="h-4 w-4" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="text-base font-semibold">帮助与节点文档</div>
            <div className="text-xs text-muted-foreground">文档来自当前后端导出的真实节点签名。</div>
          </div>
          <button onClick={close} className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground">
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="flex border-b border-border text-xs">
          {[
            ["nodes", BookOpen, "节点文档"],
            ["shortcuts", Keyboard, "快捷键"],
            ["recipes", Workflow, "工作流建议"],
          ].map(([id, Icon, label]) => (
            <button
              key={id as string}
              onClick={() => setTab(id as Tab)}
              className={cn(
                "flex items-center gap-1 px-4 py-2 transition-colors",
                tab === id ? "border-b-2 border-primary text-primary" : "text-muted-foreground hover:text-foreground"
              )}
            >
              <Icon className="h-3.5 w-3.5" />
              {label as string}
            </button>
          ))}
        </div>

        {tab === "nodes" && (
          <div className="grid min-h-0 flex-1 grid-cols-[280px_1fr]">
            <aside className="min-h-0 overflow-y-auto border-r border-border p-3">
              <div className="mb-2 flex items-center gap-2 rounded-md border border-border bg-background px-2 py-1.5">
                <Search className="h-3.5 w-3.5 text-muted-foreground" />
                <input
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder="搜索节点..."
                  className="min-w-0 flex-1 bg-transparent text-xs focus:outline-none"
                />
              </div>
              <div className="space-y-1">
                {filtered.map((d) => (
                  <button
                    key={d.id}
                    onClick={() => setSelectedId(d.id)}
                    className={cn(
                      "w-full rounded-md px-2 py-1.5 text-left transition-colors",
                      selected?.id === d.id ? "bg-primary/10 text-primary" : "hover:bg-accent"
                    )}
                  >
                    <div className="truncate text-xs font-medium">{d.displayName}</div>
                    <div className="truncate text-[10px] text-muted-foreground">{d.category} · {d.id}</div>
                  </button>
                ))}
              </div>
            </aside>
            <main className="min-h-0 overflow-y-auto p-5">
              {selected ? (
                <NodeDoc descriptor={selected} />
              ) : (
                <div className="text-sm text-muted-foreground">没有匹配节点。</div>
              )}
            </main>
          </div>
        )}

        {tab === "shortcuts" && <ShortcutDoc />}
        {tab === "recipes" && <RecipeDoc />}
      </div>
    </div>
  );
}

function NodeDoc({ descriptor }: { descriptor: ReturnType<typeof useDescriptorStore.getState>["list"][number] }) {
  return (
    <div>
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="text-xl font-semibold">{descriptor.displayName}</div>
          <div className="mt-1 font-mono text-xs text-muted-foreground">{descriptor.id}</div>
        </div>
        <span className="rounded bg-secondary px-2 py-1 text-xs text-muted-foreground">{descriptor.category}</span>
      </div>
      {descriptor.description && (
        <p className="mt-4 max-w-3xl text-sm leading-relaxed text-muted-foreground">{descriptor.description}</p>
      )}
      <div className="mt-5 grid gap-4 lg:grid-cols-2">
        <PortSection title="输入端口" ports={descriptor.inputs} empty="无输入端口" />
        <PortSection title="输出端口" ports={descriptor.outputs} empty="无输出端口" />
      </div>
      <div className="mt-5">
        <div className="mb-2 flex items-center gap-2 text-sm font-semibold">
          <Cable className="h-4 w-4 text-primary" />
          参数
        </div>
        {descriptor.params.length === 0 ? (
          <div className="rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">无参数</div>
        ) : (
          <div className="divide-y divide-border overflow-hidden rounded-md border border-border">
            {descriptor.params.map((p) => (
              <div key={p.name} className="grid grid-cols-[160px_1fr_180px] gap-3 px-3 py-2 text-xs">
                <div className="font-medium">{p.label}</div>
                <div className="font-mono text-muted-foreground">{p.name}</div>
                <div className="text-muted-foreground">{widgetText(p.widget)}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function PortSection({
  title,
  ports,
  empty,
}: {
  title: string;
  ports: { name: string; label: string; type: string; required: boolean }[];
  empty: string;
}) {
  return (
    <section>
      <div className="mb-2 text-sm font-semibold">{title}</div>
      {ports.length === 0 ? (
        <div className="rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">{empty}</div>
      ) : (
        <div className="divide-y divide-border overflow-hidden rounded-md border border-border">
          {ports.map((p) => (
            <div key={p.name} className="grid grid-cols-[120px_1fr_80px] gap-2 px-3 py-2 text-xs">
              <div className="font-medium">{p.label}</div>
              <div className="font-mono text-muted-foreground">{p.name}</div>
              <div className="text-muted-foreground">{p.type}{p.required ? "" : "?"}</div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function widgetText(widget: { kind: string; options?: string[]; min?: number; max?: number }) {
  if (widget.kind === "select") return `下拉：${widget.options?.join(" / ") ?? ""}`;
  if (widget.kind === "number" || widget.kind === "slider") return `${widget.kind} ${widget.min ?? ""}-${widget.max ?? ""}`;
  return widget.kind;
}

function ShortcutDoc() {
  const rows = [
    ["Ctrl+K", "打开命令面板"],
    ["Ctrl+N / Ctrl+O / Ctrl+S", "新建、打开、保存流程"],
    ["Ctrl+C / Ctrl+V / Ctrl+X", "复制、粘贴、剪切选中节点"],
    ["Ctrl+D", "复制选中子图"],
    ["Ctrl+Z / Ctrl+Y", "撤销、重做"],
    ["Delete / Backspace", "删除选中节点或连线"],
    ["双击画布", "打开节点搜索"],
    ["右键画布/节点/连线", "打开上下文菜单"],
  ];
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-5">
      <div className="mb-4 text-lg font-semibold">快捷键</div>
      <div className="divide-y divide-border overflow-hidden rounded-md border border-border">
        {rows.map(([key, desc]) => (
          <div key={key} className="grid grid-cols-[220px_1fr] px-3 py-2 text-sm">
            <kbd className="font-mono text-xs text-primary">{key}</kbd>
            <span className="text-muted-foreground">{desc}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function RecipeDoc() {
  const recipes = [
    ["编码链路", "文本输入 → 自动解码 / 循环解码 → 正则提取 → 文本输出"],
    ["样本文件", "资源库加入样本 → 文件导入 → 文件类型识别 / EXIF / 压缩包解包"],
    ["图像隐写", "图片输入 → 通道拆分 / 位平面 / LSB 提取 → 文本输出"],
    ["调试子图", "选中目标节点 → Inspector 中点击“运行到此处” → 沿边查看值预览"],
    ["复用流程", "框选稳定子图 → 封装为模块 → 在模块库中复用或单独调用"],
  ];
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-5">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <div className="text-lg font-semibold">工作流建议</div>
          <div className="text-xs text-muted-foreground">把常见分析动作拆成可组合的数据流。</div>
        </div>
        <Button size="sm" variant="outline" onClick={() => useHelpStore.getState().setOpen(false)}>
          明白了
        </Button>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        {recipes.map(([title, body]) => (
          <div key={title} className="rounded-lg border border-border bg-background p-3">
            <div className="text-sm font-semibold">{title}</div>
            <div className="mt-1 text-xs leading-relaxed text-muted-foreground">{body}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
