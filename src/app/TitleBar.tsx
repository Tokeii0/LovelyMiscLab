import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Boxes,
  Circle,
  Copy,
  FileCode2,
  FilePlus,
  FolderOpen,
  HelpCircle,
  Minus,
  Moon,
  Pause,
  Pencil,
  Play,
  Redo2,
  Save,
  Settings,
  Sparkles,
  Square,
  Sun,
  Trash2,
  Undo2,
  X,
} from "lucide-react";

import logo from "@/assets/logo.svg";
import { cn } from "@/lib/utils";
import { inTauri } from "@/lib/devMocks";
import { newFlow, openFlow, saveFlow } from "@/lib/project";
import { pauseRun, stopRun } from "@/flow/runner";
import { useAiStore } from "@/store/ai";
import { useHelpStore } from "@/store/help";
import { useProjectStore } from "@/store/project";
import { useGraphStore } from "@/store/graph";
import { useModuleDialogStore } from "@/store/moduleDialog";
import { useScriptDialogStore } from "@/store/scriptDialog";
import { useRunStore } from "@/store/run";
import { useThemeStore } from "@/store/theme";
import { useViewStore } from "@/store/view";

function IconButton({
  onClick,
  title,
  children,
  className,
}: {
  onClick?: () => void;
  title?: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className={cn(
        "flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground",
        className
      )}
    >
      {children}
    </button>
  );
}

export function TitleBar() {
  const theme = useThemeStore((s) => s.theme);
  const toggleTheme = useThemeStore((s) => s.toggle);
  const mode = useRunStore((s) => s.mode);
  const setMode = useRunStore((s) => s.setMode);
  const clear = useGraphStore((s) => s.clear);
  const undo = useGraphStore((s) => s.undo);
  const redo = useGraphStore((s) => s.redo);
  const canUndo = useGraphStore((s) => s.past.length > 0);
  const canRedo = useGraphStore((s) => s.future.length > 0);
  const selectedCount = useGraphStore((s) => s.nodes.reduce((a, n) => a + (n.selected ? 1 : 0), 0));
  const setView = useViewStore((s) => s.setView);
  const projectName = useProjectStore((s) => s.name);
  const renameProject = () => {
    const next = window.prompt("流程名称", projectName);
    if (next && next.trim()) useProjectStore.getState().setName(next.trim());
  };

  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    if (!inTauri) return;
    const w = getCurrentWindow();
    w.isMaximized().then(setMaximized).catch(() => {});
    const unlisten = w.onResized(() => {
      w.isMaximized().then(setMaximized).catch(() => {});
    });
    return () => {
      unlisten.then((f) => f()).catch(() => {});
    };
  }, []);

  const status =
    mode === "live"
      ? { text: "实时运行中", color: "#22c55e" }
      : mode === "paused"
        ? { text: "已暂停", color: "#f59e0b" }
        : { text: "就绪", color: "#94a3b8" };

  const ctrl =
    "flex h-8 w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground";

  return (
    <div className="flex h-11 shrink-0 items-center gap-2 border-b border-border bg-card pl-3 pr-1">
      {/* brand + file actions */}
      <div className="flex items-center gap-2">
        <img src={logo} alt="LovelyMiscLab" className="h-6 w-6 rounded-md" />
        <span className="text-sm font-semibold">LovelyMiscLab</span>
      </div>
      <div className="mx-1 h-4 w-px bg-border" />
      <div className="flex items-center gap-0.5">
        <button
          onClick={newFlow}
          title="新建 (Ctrl+N)"
          className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          <FilePlus className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={() => void openFlow()}
          title="打开 (Ctrl+O)"
          className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          <FolderOpen className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={() => void saveFlow()}
          title="保存 (Ctrl+S)"
          className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        >
          <Save className="h-3.5 w-3.5" />
        </button>
      </div>
      <button
        onClick={renameProject}
        title="重命名流程"
        className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent"
      >
        {projectName} <Pencil className="h-3 w-3" />
      </button>
      <span
        className="flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px]"
        style={{ borderColor: `${status.color}55`, color: status.color }}
      >
        <Circle className="h-2 w-2 fill-current" />
        {status.text}
      </span>

      {/* run controls (centered) */}
      <div className="flex flex-1 items-center justify-center gap-1" data-tauri-drag-region>
        <div className="flex items-center gap-1 rounded-lg border border-border bg-background p-0.5">
          <button
            onClick={() => setMode("live")}
            disabled={mode === "live"}
            className="flex items-center gap-1 rounded-md bg-primary px-3 py-1 text-xs font-medium text-primary-foreground transition hover:bg-primary/90 disabled:opacity-50"
          >
            <Play className="h-3.5 w-3.5" /> 运行
          </button>
          <button
            onClick={() => void pauseRun()}
            disabled={mode !== "live"}
            className="flex items-center gap-1 rounded-md px-2.5 py-1 text-xs text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40"
          >
            <Pause className="h-3.5 w-3.5" /> 暂停
          </button>
          <button
            onClick={() => void stopRun()}
            disabled={mode === "idle"}
            className="flex items-center gap-1 rounded-md px-2.5 py-1 text-xs text-muted-foreground hover:bg-destructive/10 hover:text-destructive disabled:opacity-40"
          >
            <Square className="h-3.5 w-3.5" /> 停止
          </button>
        </div>
        <button
          onClick={clear}
          className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          <Trash2 className="h-3.5 w-3.5" /> 清空
        </button>
      </div>

      {/* encapsulate selection into a reusable module */}
      <button
        onClick={() => useModuleDialogStore.getState().setOpen(true)}
        disabled={selectedCount === 0}
        className="flex items-center gap-1 rounded-md px-2.5 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:opacity-40"
        title={selectedCount === 0 ? "先在画布上选择节点" : `把选中的 ${selectedCount} 个节点封装为模块`}
      >
        <Boxes className="h-3.5 w-3.5" /> 封装
      </button>

      {/* create external-script node */}
      <button
        onClick={() => useScriptDialogStore.getState().setOpen(true)}
        className="flex items-center gap-1 rounded-md px-2.5 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        title="把外部脚本/程序接入为节点"
      >
        <FileCode2 className="h-3.5 w-3.5" /> 脚本节点
      </button>

      {/* AI generate */}
      <button
        onClick={() => useAiStore.getState().setOpen(true)}
        className="flex items-center gap-1 rounded-md bg-primary/10 px-2.5 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/15"
        title="AI 一键生成流程"
      >
        <Sparkles className="h-3.5 w-3.5" /> AI 生成
      </button>

      {/* right utilities */}
      <div className="ml-1 flex items-center gap-0.5">
        <IconButton
          title={canUndo ? "撤销 (Ctrl+Z)" : "暂无可撤销操作"}
          onClick={undo}
          className={!canUndo ? "opacity-40" : undefined}
        >
          <Undo2 className="h-4 w-4" />
        </IconButton>
        <IconButton
          title={canRedo ? "重做 (Ctrl+Shift+Z)" : "暂无可重做操作"}
          onClick={redo}
          className={!canRedo ? "opacity-40" : undefined}
        >
          <Redo2 className="h-4 w-4" />
        </IconButton>
        <div className="mx-1 h-4 w-px bg-border" />
        <IconButton title="切换主题" onClick={toggleTheme}>
          {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </IconButton>
        <IconButton title="帮助" onClick={() => useHelpStore.getState().openForNode()}>
          <HelpCircle className="h-4 w-4" />
        </IconButton>
        <IconButton title="设置" onClick={() => setView("settings")}>
          <Settings className="h-4 w-4" />
        </IconButton>
      </div>

      {/* window controls */}
      {inTauri && (
        <div className="flex items-stretch">
          <button
            className={ctrl}
            title="最小化"
            onClick={() => getCurrentWindow().minimize()}
          >
            <Minus className="h-4 w-4" />
          </button>
          <button
            className={ctrl}
            title="最大化 / 还原"
            onClick={() => getCurrentWindow().toggleMaximize()}
          >
            {maximized ? <Copy className="h-3.5 w-3.5" /> : <Square className="h-3.5 w-3.5" />}
          </button>
          <button
            className="flex h-8 w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-destructive hover:text-destructive-foreground"
            title="关闭"
            onClick={() => getCurrentWindow().close()}
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      )}
    </div>
  );
}
