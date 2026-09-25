import { useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Circle,
  Copy,
  FilePlus,
  FolderOpen,
  HelpCircle,
  Minus,
  Moon,
  MoreHorizontal,
  Pause,
  Pencil,
  Play,
  Redo2,
  Save,
  Sparkles,
  Square,
  Sun,
  Undo2,
  X,
} from "lucide-react";

import logo from "@/assets/logo.svg";
import { cn } from "@/lib/utils";
import { inTauri } from "@/lib/devMocks";
import { newFlow, openFlow, renameFlow, saveFlow } from "@/lib/project";
import { clearCanvas } from "@/flow/canvasActions";
import { ContextMenu, type MenuItem } from "@/flow/ContextMenu";
import { pauseRun, stopRun } from "@/flow/runner";
import { useWindowMaximized } from "@/hooks/useWindowMaximized";
import { useAiStore } from "@/store/ai";
import { promptDialog } from "@/store/confirm";
import { useGraphStore } from "@/store/graph";
import { useHelpStore } from "@/store/help";
import { useModuleDialogStore } from "@/store/moduleDialog";
import { useProjectDirty, useProjectStore } from "@/store/project";
import { useRunStore } from "@/store/run";
import { useScriptDialogStore } from "@/store/scriptDialog";
import { useThemeStore } from "@/store/theme";
import { useViewStore } from "@/store/view";

function IconButton({
  onClick,
  title,
  children,
  disabled,
}: {
  onClick?: () => void;
  title?: string;
  children: React.ReactNode;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      disabled={disabled}
      className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
    >
      {children}
    </button>
  );
}

/** Canvas-only "more" actions: encapsulate, script node, clear. */
function MoreMenu() {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const btn = useRef<HTMLButtonElement>(null);
  const selectedCount = useGraphStore((s) => s.nodes.reduce((a, n) => a + (n.selected ? 1 : 0), 0));

  const items: MenuItem[] = [
    {
      label: selectedCount ? `封装选中的 ${selectedCount} 个节点为模块…` : "封装为模块…（先选择节点）",
      disabled: selectedCount === 0,
      onClick: () => useModuleDialogStore.getState().setOpen(true),
    },
    { label: "接入外部脚本为节点…", onClick: () => useScriptDialogStore.getState().setOpen(true) },
    { label: "清空画布…", danger: true, onClick: () => void clearCanvas() },
  ];

  return (
    <>
      <button
        ref={btn}
        title="更多画布操作"
        onClick={() => {
          const r = btn.current?.getBoundingClientRect();
          if (r) setAt({ x: r.right - 220, y: r.bottom + 4 });
        }}
        className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
      >
        <MoreHorizontal className="h-4 w-4" />
      </button>
      {at && <ContextMenu x={at.x} y={at.y} items={items} onClose={() => setAt(null)} />}
    </>
  );
}

export function TitleBar() {
  const theme = useThemeStore((s) => s.theme);
  const toggleTheme = useThemeStore((s) => s.toggle);
  const mode = useRunStore((s) => s.mode);
  const setMode = useRunStore((s) => s.setMode);
  const undo = useGraphStore((s) => s.undo);
  const redo = useGraphStore((s) => s.redo);
  const canUndo = useGraphStore((s) => s.past.length > 0);
  const canRedo = useGraphStore((s) => s.future.length > 0);
  const onCanvas = useViewStore((s) => s.view === "canvas");
  const projectName = useProjectStore((s) => s.name);
  const dirty = useProjectDirty();
  const maximized = useWindowMaximized();

  const renameProject = async () => {
    const next = await promptDialog({ title: "重命名流程", initial: projectName, confirmText: "重命名" });
    if (next != null) renameFlow(next);
  };

  const status =
    mode === "live"
      ? { text: "实时运行中", color: "#22c55e" }
      : mode === "paused"
        ? { text: "已暂停", color: "#f59e0b" }
        : { text: "就绪", color: "#94a3b8" };

  const ctrl =
    "flex h-8 w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground";

  return (
    // The whole bar drags the window; only elements carrying the attribute
    // themselves count, so buttons stay clickable.
    <div
      data-tauri-drag-region
      className="flex h-11 shrink-0 items-center gap-2 border-b border-border bg-card pl-3 pr-1"
    >
      {/* brand + file actions (work from every view) */}
      <div data-tauri-drag-region className="flex shrink-0 items-center gap-2">
        <img src={logo} alt="" data-tauri-drag-region className="h-6 w-6 rounded-md" />
        <span data-tauri-drag-region className="text-sm font-semibold">
          LovelyMiscLab
        </span>
      </div>
      <div className="mx-1 h-4 w-px shrink-0 bg-border" />
      <div className="flex shrink-0 items-center gap-0.5">
        <IconButton title="新建 (Ctrl+N)" onClick={() => void newFlow()}>
          <FilePlus className="h-3.5 w-3.5" />
        </IconButton>
        <IconButton title="打开 (Ctrl+O)" onClick={() => void openFlow()}>
          <FolderOpen className="h-3.5 w-3.5" />
        </IconButton>
        <IconButton title="保存 (Ctrl+S，另存为 Ctrl+Shift+S)" onClick={() => void saveFlow()}>
          <Save className="h-3.5 w-3.5" />
        </IconButton>
      </div>
      <button
        onClick={() => void renameProject()}
        title={dirty ? "有未保存的修改 · 点击重命名" : "重命名流程"}
        className="flex min-w-0 max-w-[220px] items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent"
      >
        <span className="truncate">{projectName}</span>
        {dirty && <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-amber-500" />}
        <Pencil className="h-3 w-3 shrink-0" />
      </button>

      {/* run controls — canvas only */}
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center justify-center gap-2">
        {onCanvas && (
          <>
            <span
              className="flex shrink-0 items-center gap-1 rounded-full border px-2 py-0.5 text-[11px]"
              style={{ borderColor: `${status.color}55`, color: status.color }}
            >
              <Circle className="h-2 w-2 fill-current" />
              {status.text}
            </span>
            <div className="flex shrink-0 items-center gap-1 rounded-lg border border-border bg-background p-0.5">
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
          </>
        )}
      </div>

      {onCanvas && (
        <div className="flex shrink-0 items-center gap-0.5">
          <button
            onClick={() => useAiStore.getState().setOpen(true)}
            className="mr-1 flex items-center gap-1 rounded-md bg-primary/10 px-2.5 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/15"
            title="AI 生成 / 解释 / 修复流程"
          >
            <Sparkles className="h-3.5 w-3.5" /> AI 助手
          </button>
          <IconButton title="撤销 (Ctrl+Z)" onClick={undo} disabled={!canUndo}>
            <Undo2 className="h-4 w-4" />
          </IconButton>
          <IconButton title="重做 (Ctrl+Shift+Z)" onClick={redo} disabled={!canRedo}>
            <Redo2 className="h-4 w-4" />
          </IconButton>
          <MoreMenu />
        </div>
      )}

      {/* app utilities */}
      <div className="flex shrink-0 items-center gap-0.5">
        <div className="mx-1 h-4 w-px bg-border" />
        <IconButton title={theme === "dark" ? "切换到浅色主题" : "切换到深色主题"} onClick={toggleTheme}>
          {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </IconButton>
        <IconButton title="帮助" onClick={() => useHelpStore.getState().openForNode()}>
          <HelpCircle className="h-4 w-4" />
        </IconButton>
      </div>

      {/* window controls */}
      {inTauri && (
        <div className="flex shrink-0 items-stretch">
          <button className={ctrl} title="最小化" onClick={() => getCurrentWindow().minimize()}>
            <Minus className="h-4 w-4" />
          </button>
          <button
            className={ctrl}
            title={maximized ? "还原" : "最大化"}
            onClick={() => getCurrentWindow().toggleMaximize()}
          >
            {maximized ? <Copy className="h-3.5 w-3.5" /> : <Square className="h-3.5 w-3.5" />}
          </button>
          <button
            className={cn(ctrl, "hover:bg-destructive hover:text-destructive-foreground")}
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
