import { useEffect, useMemo, useRef, useState } from "react";
import {
  Boxes,
  Bot,
  FilePlus,
  FolderOpen,
  HelpCircle,
  History,
  LayoutGrid,
  Package,
  Pause,
  Play,
  Save,
  Search,
  Settings,
  Square,
  Workflow,
  X,
  type LucideIcon,
} from "lucide-react";

import { newFlow, openFlow, saveFlow } from "@/lib/project";
import { cn } from "@/lib/utils";
import { executeGraph, stopRun, pauseRun } from "@/flow/runner";
import { nodeIcon } from "@/flow/nodeIcons";
import { useAiStore } from "@/store/ai";
import { useCommandPaletteStore } from "@/store/commandPalette";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useHelpStore } from "@/store/help";
import { useRunStore } from "@/store/run";
import { useViewStore, type View } from "@/store/view";

interface Command {
  id: string;
  title: string;
  hint: string;
  icon: LucideIcon;
  keywords: string;
  action: () => void;
}

const viewCommands: { view: View; title: string; icon: LucideIcon }[] = [
  { view: "canvas", title: "打开画布", icon: LayoutGrid },
  { view: "modules", title: "打开模块库", icon: Boxes },
  { view: "templates", title: "打开模板", icon: Workflow },
  { view: "runs", title: "打开运行记录", icon: History },
  { view: "resources", title: "打开资源库", icon: Package },
  { view: "settings", title: "打开设置", icon: Settings },
];

export function CommandPalette() {
  const open = useCommandPaletteStore((s) => s.open);
  const setOpen = useCommandPaletteStore((s) => s.setOpen);
  const descriptors = useDescriptorStore((s) => s.list);
  const addNode = useGraphStore((s) => s.addNode);
  const setView = useViewStore((s) => s.setView);
  const setRunMode = useRunStore((s) => s.setMode);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActive(0);
    setTimeout(() => inputRef.current?.focus(), 0);
  }, [open]);

  const commands = useMemo<Command[]>(() => {
    const base: Command[] = [
      {
        id: "new",
        title: "新建流程",
        hint: "清空当前画布并创建新流程",
        icon: FilePlus,
        keywords: "new flow",
        action: newFlow,
      },
      {
        id: "open",
        title: "打开流程文件",
        hint: "选择 .lml 或 .json 流程文件",
        icon: FolderOpen,
        keywords: "open file",
        action: () => void openFlow(),
      },
      {
        id: "save",
        title: "保存流程",
        hint: "保存当前画布",
        icon: Save,
        keywords: "save file",
        action: () => void saveFlow(),
      },
      {
        id: "run",
        title: "运行整图",
        hint: "手动执行当前工作流",
        icon: Play,
        keywords: "run execute",
        action: () => void executeGraph(),
      },
      {
        id: "live",
        title: "开启实时模式",
        hint: "参数或连线变化后自动增量运行",
        icon: Play,
        keywords: "live auto",
        action: () => setRunMode("live"),
      },
      {
        id: "pause",
        title: "暂停运行",
        hint: "取消当前任务并保留结果",
        icon: Pause,
        keywords: "pause",
        action: () => void pauseRun(),
      },
      {
        id: "stop",
        title: "停止并清空运行状态",
        hint: "取消任务、清缓存、清节点输出",
        icon: Square,
        keywords: "stop reset",
        action: () => void stopRun(),
      },
      {
        id: "ai",
        title: "AI 生成/解释流程",
        hint: "打开 AI 工作流助手",
        icon: Bot,
        keywords: "ai generate explain repair",
        action: () => useAiStore.getState().setOpen(true),
      },
      {
        id: "help",
        title: "帮助与节点文档",
        hint: "查看节点签名、快捷键和工作流建议",
        icon: HelpCircle,
        keywords: "help docs shortcut",
        action: () => useHelpStore.getState().openForNode(),
      },
      ...viewCommands.map((v) => ({
        id: `view-${v.view}`,
        title: v.title,
        hint: "切换主视图",
        icon: v.icon,
        keywords: `view ${v.view}`,
        action: () => setView(v.view),
      })),
    ];
    const nodeCommands: Command[] = descriptors.map((d) => ({
      id: `node-${d.id}`,
      title: `添加节点：${d.displayName}`,
      hint: `${d.category} · ${d.id}`,
      icon: nodeIcon(d.id, d.category),
      keywords: `${d.id} ${d.displayName} ${d.category} ${d.description ?? ""}`,
      action: () => {
        addNode(d, { x: 260 + Math.random() * 120, y: 160 + Math.random() * 120 });
        setView("canvas");
      },
    }));
    return [...base, ...nodeCommands];
  }, [addNode, descriptors, setRunMode, setView]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands.slice(0, 24);
    return commands
      .filter((c) => `${c.title} ${c.hint} ${c.keywords}`.toLowerCase().includes(q))
      .slice(0, 36);
  }, [commands, query]);

  if (!open) return null;

  const run = (command: Command) => {
    command.action();
    setOpen(false);
  };

  return (
    <div className="fixed inset-0 z-[90] bg-black/35 p-4 pt-[12vh]" onClick={() => setOpen(false)}>
      <div
        className="mx-auto flex max-h-[72vh] w-[720px] max-w-[96vw] flex-col overflow-hidden rounded-lg border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border px-3 py-2">
          <Search className="h-4 w-4 text-muted-foreground" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setActive(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                setActive((i) => Math.min(i + 1, filtered.length - 1));
                e.preventDefault();
              } else if (e.key === "ArrowUp") {
                setActive((i) => Math.max(i - 1, 0));
                e.preventDefault();
              } else if (e.key === "Enter" && filtered[active]) {
                run(filtered[active]);
                e.preventDefault();
              } else if (e.key === "Escape") {
                setOpen(false);
              }
            }}
            placeholder="输入命令、节点名或视图..."
            className="min-w-0 flex-1 bg-transparent py-2 text-sm focus:outline-none"
          />
          <button className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground" onClick={() => setOpen(false)}>
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-2">
          {filtered.length === 0 ? (
            <div className="p-4 text-center text-xs text-muted-foreground">没有匹配命令</div>
          ) : (
            filtered.map((command, index) => {
              const Icon = command.icon;
              return (
                <button
                  key={command.id}
                  onMouseEnter={() => setActive(index)}
                  onClick={() => run(command)}
                  className={cn(
                    "flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors",
                    index === active ? "bg-primary/10 text-primary" : "hover:bg-accent"
                  )}
                >
                  <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-secondary text-muted-foreground">
                    <Icon className="h-4 w-4" />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-medium">{command.title}</span>
                    <span className="block truncate text-xs text-muted-foreground">{command.hint}</span>
                  </span>
                </button>
              );
            })
          )}
        </div>
        <div className="border-t border-border px-3 py-2 text-[10px] text-muted-foreground">
          ↑↓ 选择 · Enter 执行 · Esc 关闭
        </div>
      </div>
    </div>
  );
}
