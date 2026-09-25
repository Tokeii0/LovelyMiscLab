import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Boxes,
  Bot,
  Eraser,
  FilePlus,
  FolderOpen,
  Gamepad2,
  HelpCircle,
  History,
  LayoutGrid,
  Package,
  Play,
  Save,
  Search,
  Settings,
  Square,
  Workflow,
  X,
  Zap,
  type LucideIcon,
} from "lucide-react";

import { newFlow, openFlow, saveFlow, saveFlowAs } from "@/lib/project";
import { cn } from "@/lib/utils";
import { clearResults, runGraph, setLiveMode, stopRun } from "@/flow/runner";
import { nodeIcon } from "@/flow/nodeIcons";
import { useAiStore } from "@/store/ai";
import { useCommandPaletteStore } from "@/store/commandPalette";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useHelpStore } from "@/store/help";
import { usePrefs } from "@/store/prefs";
import { useRunStore } from "@/store/run";
import { useViewStore, VIEW_LABEL, type View } from "@/store/view";
import { useScrollActiveIntoView } from "@/hooks/useScrollActiveIntoView";
import { useEscapeToClose } from "@/store/modal";
import { nodeSummary } from "@/flow/nodeDescriptions";
import { placeInView } from "@/flow/placement";
import { searchDescriptors } from "@/lib/nodeSearch";
import type { NodeDescriptor } from "@/lib/types";

interface Command {
  id: string;
  title: string;
  hint: string;
  icon: LucideIcon;
  keywords: string;
  action: () => void;
}

const viewCommands: { view: View; icon: LucideIcon }[] = [
  { view: "canvas", icon: LayoutGrid },
  { view: "modules", icon: Boxes },
  { view: "templates", icon: Workflow },
  { view: "runs", icon: History },
  { view: "resources", icon: Package },
  { view: "settings", icon: Settings },
  { view: "galgame", icon: Gamepad2 },
];

export function CommandPalette() {
  const open = useCommandPaletteStore((s) => s.open);
  const setOpen = useCommandPaletteStore((s) => s.setOpen);
  const descriptors = useDescriptorStore((s) => s.list);
  const addNode = useGraphStore((s) => s.addNode);
  const setView = useViewStore((s) => s.setView);
  const galgame = usePrefs((s) => s.experimentalGalgame);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

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
        action: () => void newFlow(),
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
        id: "saveAs",
        title: "另存为…",
        hint: "保存为新的流程文件 (Ctrl+Shift+S)",
        icon: Save,
        keywords: "save as",
        action: () => void saveFlowAs(),
      },
      {
        id: "run",
        title: "运行整图",
        hint: "执行一次当前工作流 (Ctrl+Enter)",
        icon: Play,
        keywords: "run execute",
        action: () => {
          setView("canvas");
          void runGraph();
        },
      },
      {
        id: "live",
        title: "切换实时模式",
        hint: "开启后，参数或连线变化会自动增量运行（耗时节点除外）",
        icon: Zap,
        keywords: "live auto realtime",
        action: () => setLiveMode(useRunStore.getState().mode !== "live"),
      },
      {
        id: "stop",
        title: "停止运行",
        hint: "取消正在进行的运行，已有结果保留",
        icon: Square,
        keywords: "stop cancel",
        action: () => void stopRun(),
      },
      {
        id: "clear-results",
        title: "清除运行结果",
        hint: "清空所有节点的输出与缓存",
        icon: Eraser,
        keywords: "clear reset results cache",
        action: () => void clearResults(),
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
      ...viewCommands
        .filter((v) => v.view !== "galgame" || galgame)
        .map((v) => ({
        id: `view-${v.view}`,
        title: `前往「${VIEW_LABEL[v.view]}」`,
        hint: "切换主视图",
        icon: v.icon,
        keywords: `view ${v.view}`,
        action: () => setView(v.view),
      })),
    ];
    return base;
  }, [galgame, setView]);

  const nodeCommand = useCallback(
    (d: NodeDescriptor): Command => ({
      id: `node-${d.id}`,
      title: `添加节点：${d.displayName}`,
      hint: `${d.category} · ${nodeSummary(d) || d.id}`,
      icon: nodeIcon(d.id, d.category),
      keywords: d.id,
      action: () => {
        setView("canvas");
        addNode(d, placeInView());
      },
    }),
    [addNode, setView]
  );

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands.slice(0, 24);
    const tokens = q.split(/\s+/);
    const cmds = commands.filter((c) => {
      const hay = `${c.title} ${c.hint} ${c.keywords}`.toLowerCase();
      return tokens.every((t) => hay.includes(t));
    });
    // Nodes use the same search (and ranking) as the library and search menu.
    const nodes = searchDescriptors(descriptors, q).slice(0, 30).map(nodeCommand);
    return [...cmds, ...nodes].slice(0, 40);
  }, [commands, descriptors, nodeCommand, query]);

  const close = () => setOpen(false);
  useEscapeToClose(open, close);
  useScrollActiveIntoView(listRef, active);

  if (!open) return null;

  const run = (command: Command) => {
    command.action();
    setOpen(false);
  };

  return (
    <div className="fixed inset-0 z-[90] bg-black/35 p-4 pt-[12vh]" onClick={close}>
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
              if (e.nativeEvent.isComposing) return;
              if (e.key === "ArrowDown") {
                setActive((i) => Math.min(i + 1, filtered.length - 1));
                e.preventDefault();
              } else if (e.key === "ArrowUp") {
                setActive((i) => Math.max(i - 1, 0));
                e.preventDefault();
              } else if (e.key === "Enter" && filtered[active]) {
                run(filtered[active]);
                e.preventDefault();
              }
            }}
            placeholder="输入命令、节点名或视图..."
            className="min-w-0 flex-1 bg-transparent py-2 text-sm focus:outline-none"
          />
          <button
            className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
            onClick={() => setOpen(false)}
          >
            <X className="h-4 w-4" />
          </button>
        </div>
        <div ref={listRef} className="min-h-0 flex-1 overflow-y-auto p-2">
          {filtered.length === 0 ? (
            <div className="p-4 text-center text-xs text-muted-foreground">没有匹配命令</div>
          ) : (
            filtered.map((command, index) => {
              const Icon = command.icon;
              return (
                <button
                  key={command.id}
                  data-index={index}
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
                    <span className="block truncate text-xs text-muted-foreground">
                      {command.hint}
                    </span>
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
