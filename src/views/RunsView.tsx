import { useMemo, useState } from "react";
import { Clock3, History, RotateCcw, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Empty } from "@/components/ui/empty";
import { OutputValue } from "@/flow/portValue";
import type { PortValue } from "@/lib/types";
import { cn } from "@/lib/utils";
import { confirmDialog } from "@/store/confirm";
import { useDescriptorStore } from "@/store/descriptors";
import { useRunStore, type RunHistoryEntry, type RunHistoryNode, type RunStatus } from "@/store/run";

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

function nodeStatusTone(s: string) {
  return s === "done"
    ? "bg-green-500/15 text-green-600"
    : s === "error"
      ? "bg-destructive/10 text-destructive"
      : s === "running"
        ? "bg-blue-500/15 text-blue-600"
        : "bg-secondary text-muted-foreground";
}

/** The actual output ports a node produced, rendered by type. */
function NodeOutputs({ node }: { node: RunHistoryNode }) {
  const byId = useDescriptorStore((s) => s.byId);
  if (!node.outputs || Object.keys(node.outputs).length === 0) return null;
  const desc = byId[node.descriptorId];
  const specs = desc?.outputs ?? [];
  const ordered: [string, PortValue][] = [
    ...specs
      .filter((o) => node.outputs![o.name] !== undefined)
      .map((o) => [o.name, node.outputs![o.name]] as [string, PortValue]),
    ...Object.entries(node.outputs).filter(([k]) => !specs.some((o) => o.name === k)),
  ];
  return (
    <div className="mt-2 space-y-2 border-t border-border/60 pt-2">
      {ordered.map(([port, val]) => (
        <div key={port}>
          <div className="mb-0.5 text-[10px] font-medium text-muted-foreground">
            {specs.find((o) => o.name === port)?.label ?? port}
          </div>
          <OutputValue value={val} />
        </div>
      ))}
    </div>
  );
}

const NODE_STATUS_LABEL: Record<string, string> = {
  idle: "未运行",
  running: "运行中",
  done: "完成",
  error: "失败",
  skipped: "已跳过",
};

const LEVEL_LABEL: Record<string, string> = {
  info: "信息",
  success: "成功",
  warn: "警告",
  error: "错误",
  debug: "调试",
};

function RunDetail({ entry }: { entry: RunHistoryEntry }) {
  const labelOf = new Map(entry.nodes.map((n) => [n.id, n.label]));
  const failed = entry.nodes.filter((n) => n.status === "error").length;
  const done = entry.nodes.filter((n) => n.status === "done").length;
  const withOutputs = entry.nodes.filter((n) => n.outputs && Object.keys(n.outputs).length > 0);
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

      <div className="grid min-h-0 flex-1 grid-cols-[1fr_300px]">
        <div className="min-h-0 overflow-y-auto p-3">
          <div className="mb-2 text-xs font-semibold text-muted-foreground">
            节点结果与输出值
          </div>
          <div className="space-y-2">
            {entry.nodes.map((node) => (
              <div key={node.id} className="rounded-md border border-border bg-card px-2.5 py-2">
                <div className="flex items-center justify-between gap-2">
                  <span className="min-w-0 truncate text-xs font-medium">{node.label}</span>
                  <span
                    className={cn(
                      "shrink-0 rounded px-1.5 py-0.5 text-[10px]",
                      nodeStatusTone(node.status)
                    )}
                  >
                    {NODE_STATUS_LABEL[node.status] ?? node.status}
                  </span>
                </div>
                {node.error && (
                  <div className="mt-1 line-clamp-3 text-[10px] text-destructive">{node.error}</div>
                )}
                <NodeOutputs node={node} />
              </div>
            ))}
          </div>
          {withOutputs.length === 0 && !entry.error && (
            <div className="mt-2 text-[11px] text-muted-foreground">
              本次运行未捕获到输出值（可能在浏览器预览中运行，或节点无输出）。
            </div>
          )}
        </div>
        <div className="min-h-0 overflow-y-auto border-l border-border p-3">
          <div className="mb-2 flex items-center gap-1 text-xs font-semibold text-muted-foreground">
            <Clock3 className="h-3.5 w-3.5" />
            事件流
          </div>
          {entry.events.length === 0 ? (
            <div className="text-xs text-muted-foreground">暂无事件</div>
          ) : (
            <div className="space-y-1 text-[11px]">
              {entry.events.map((event, i) => (
                <div key={`${event.time}-${i}`} className="rounded px-2 py-1 hover:bg-accent/40">
                  <div className="flex items-center gap-2 text-[10px]">
                    <span className="font-mono text-muted-foreground">{event.time}</span>
                    <span
                      className={cn(
                        event.level === "error"
                          ? "text-destructive"
                          : event.level === "success"
                            ? "text-green-600"
                            : event.level === "warn"
                              ? "text-amber-600"
                              : "text-muted-foreground"
                      )}
                    >
                      {LEVEL_LABEL[event.level] ?? event.level}
                    </span>
                    {event.node && (
                      <span className="min-w-0 truncate text-muted-foreground">
                        {labelOf.get(event.node) ?? event.node}
                      </span>
                    )}
                  </div>
                  <div className="break-words">{event.message}</div>
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
        <Button
          variant="outline"
          size="sm"
          disabled={history.length === 0}
          onClick={async () => {
            const ok = await confirmDialog({
              title: "清空全部运行记录？",
              message: `将删除 ${history.length} 条记录，无法恢复。`,
              confirmText: "清空",
              danger: true,
            });
            if (ok) clearHistory();
          }}
        >
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
