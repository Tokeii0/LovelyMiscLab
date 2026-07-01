import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Circle,
  Copy,
  HelpCircle,
  Minus,
  Moon,
  Pause,
  Pencil,
  Play,
  Redo2,
  Search,
  Settings,
  Square,
  Sun,
  Trash2,
  Undo2,
  X,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { inTauri } from "@/lib/devMocks";
import { pauseRun, stopRun } from "@/flow/runner";
import { useGraphStore } from "@/store/graph";
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
  const setView = useViewStore((s) => s.setView);

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
      {/* brand + workflow */}
      <div className="flex items-center gap-2">
        <div className="flex h-6 w-6 items-center justify-center rounded-md bg-primary text-[13px] font-bold text-primary-foreground">
          K
        </div>
        <span className="text-sm font-semibold">LovelyMiscLab</span>
      </div>
      <div className="mx-1 h-4 w-px bg-border" />
      <button className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent">
        未命名流程 <Pencil className="h-3 w-3" />
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

      {/* command search */}
      <div className="flex h-7 w-56 items-center gap-2 rounded-md border border-border bg-background px-2 text-xs text-muted-foreground">
        <Search className="h-3.5 w-3.5" />
        <span className="flex-1">搜索命令…</span>
        <kbd className="rounded border border-border px-1 text-[10px]">Ctrl K</kbd>
      </div>

      {/* right utilities */}
      <div className="ml-1 flex items-center gap-0.5">
        <IconButton title="撤销">
          <Undo2 className="h-4 w-4" />
        </IconButton>
        <IconButton title="重做">
          <Redo2 className="h-4 w-4" />
        </IconButton>
        <div className="mx-1 h-4 w-px bg-border" />
        <IconButton title="切换主题" onClick={toggleTheme}>
          {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </IconButton>
        <IconButton title="帮助">
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
