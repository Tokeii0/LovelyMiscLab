import { useEffect, useState } from "react";
import { Check, Copy, Link2, Play, StepForward } from "lucide-react";

import { ProgressBar } from "@/components/ui/progress";
import { cn } from "@/lib/utils";
import type { PortValue } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore, type FlowNode } from "@/store/graph";
import { useInspectorStore, type InspectorTab as Tab } from "@/store/inspector";

import { nodeIcon } from "./nodeIcons";
import { bytesToHex, OutputValue, valueText } from "./portValue";
import { runNode, runToNode } from "./runner";
import { WidgetRenderer } from "./WidgetRenderer";

function CopyButton({ text }: { text: string }) {
  const [done, setDone] = useState(false);
  return (
    <button
      onClick={() => {
        navigator.clipboard?.writeText(text).catch(() => {});
        setDone(true);
        setTimeout(() => setDone(false), 1200);
      }}
      className="flex items-center gap-1 rounded border border-border px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-accent"
    >
      {done ? <Check className="h-3 w-3 text-green-600" /> : <Copy className="h-3 w-3" />}
      复制
    </button>
  );
}

function StatusBadge({ status }: { status: FlowNode["data"]["status"] }) {
  const map = {
    idle: { t: "空闲", c: "#94a3b8" },
    running: { t: "运行中", c: "#3b82f6" },
    done: { t: "执行成功", c: "#22c55e" },
    error: { t: "执行失败", c: "#ef4444" },
    skipped: { t: "已跳过", c: "#a3a3a3" },
  } as const;
  const s = map[status];
  return (
    <span
      className="flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px]"
      style={{ background: `${s.c}18`, color: s.c }}
    >
      {s.t}
    </span>
  );
}

/** Node name: edited as a draft, committed on blur/Enter; empty = the default name. */
function NameField({
  value,
  fallback,
  onCommit,
}: {
  value: string;
  fallback: string;
  onCommit: (v: string) => void;
}) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => {
    const next = draft.trim() || fallback;
    setDraft(next);
    if (next !== value) onCommit(next);
  };
  return (
    <input
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.nativeEvent.isComposing) return;
        if (e.key === "Enter") e.currentTarget.blur();
        else if (e.key === "Escape") setDraft(value);
      }}
      placeholder={fallback}
      title="节点名称（可修改，留空恢复默认）"
      className="min-w-0 flex-1 rounded border border-transparent bg-transparent text-sm font-semibold hover:border-border focus:border-input focus:bg-background focus:outline-none"
    />
  );
}

export function Inspector() {
  const selectedId = useGraphStore((s) => s.selectedId);
  const node = useGraphStore((s) => s.nodes.find((n) => n.id === selectedId));
  const setParam = useGraphStore((s) => s.setParam);
  const edges = useGraphStore((s) => s.edges);
  const toggleParamInput = useGraphStore((s) => s.toggleParamInput);
  const renameNode = useGraphStore((s) => s.renameNode);
  const descriptor = useDescriptorStore((s) =>
    node ? s.byId[node.data.descriptorId] : undefined
  );
  const tab = useInspectorStore((s) => s.tab);
  const setTab = useInspectorStore((s) => s.setTab);

  if (!node || !descriptor) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-xs text-muted-foreground">
        选择一个节点查看详情
      </div>
    );
  }

  const Icon = nodeIcon(descriptor.id, descriptor.category);
  const outputs = node.data.outputs ?? {};
  const logs = node.data.logs ?? [];

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border p-3">
        <div className="flex items-center gap-2">
          <span
            className="flex h-7 w-7 items-center justify-center rounded-md"
            style={{ background: `${descriptor.color}18`, color: descriptor.color }}
          >
            <Icon className="h-4 w-4" />
          </span>
          <NameField
            key={node.id}
            value={node.data.label || descriptor.displayName}
            fallback={descriptor.displayName}
            onCommit={(v) => renameNode(node.id, v)}
          />
          <div className="ml-auto shrink-0">
            <StatusBadge status={node.data.status} />
          </div>
        </div>
        <div className="mt-1.5 flex items-center gap-2 text-[10px] text-muted-foreground">
          <span className="rounded bg-secondary px-1.5 py-0.5">{descriptor.category}</span>
          <span className="font-mono">{node.id}</span>
        </div>
        <div className="mt-2 flex gap-1">
          <button
            onClick={() => void runNode(node.id)}
            className="flex items-center gap-1 rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            <Play className="h-3 w-3" />
            运行此节点
          </button>
          <button
            onClick={() => void runToNode(node.id)}
            className="flex items-center gap-1 rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            <StepForward className="h-3 w-3" />
            运行到此（含上游）
          </button>
        </div>
        {node.data.hint && node.data.status !== "running" && (
          <div className="mt-2 rounded bg-secondary px-2 py-1 text-[10px] text-muted-foreground">
            {node.data.hint}
          </div>
        )}
        {node.data.status === "running" && (
          <div className="mt-2">
            <ProgressBar
              value={node.data.progress ?? 0}
              status={logs[logs.length - 1]?.message}
            />
          </div>
        )}
      </div>

      <div className="flex border-b border-border text-xs">
        {(
          [
            ["params", "参数"],
            ["output", "输出"],
            ["logs", "日志"],
          ] as [Tab, string][]
        ).map(([t, label]) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={cn(
              "flex-1 py-2 transition-colors",
              tab === t
                ? "border-b-2 border-primary font-medium text-primary"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto p-3 text-xs">
        {tab === "params" && (
          <>
            <div className="mb-2 text-[11px] font-semibold text-muted-foreground">
              基础设置
            </div>
            {descriptor.params.length === 0 ? (
              <div className="text-muted-foreground">该节点无可配置参数</div>
            ) : (
              descriptor.params.map((p) => {
                const promoted = node.data.inputParams?.includes(p.name) ?? false;
                const connected = edges.some(
                  (e) => e.target === node.id && e.targetHandle === p.name
                );
                return (
                  <div key={p.name} className="mb-3">
                    <div className="mb-0.5 flex items-center justify-between">
                      <span className="text-[11px] text-muted-foreground">{p.label}</span>
                      <button
                        onClick={() => toggleParamInput(node.id, p.name)}
                        title={promoted ? "转回参数" : "转为输入（可连接节点驱动）"}
                        className={cn(
                          "flex items-center gap-0.5 rounded px-1.5 py-0.5 text-[9px] transition-colors",
                          promoted
                            ? "bg-primary/15 text-primary"
                            : "text-muted-foreground hover:bg-accent hover:text-foreground"
                        )}
                      >
                        <Link2 className="h-2.5 w-2.5" />
                        {promoted ? "输入" : "转输入"}
                      </button>
                    </div>
                    {promoted && connected ? (
                      <div className="rounded border border-dashed border-primary/40 bg-primary/5 px-2 py-1 text-[10px] text-primary">
                        由上游连接提供
                      </div>
                    ) : (
                      <>
                        <WidgetRenderer
                          spec={p}
                          value={node.data.params[p.name]}
                          onChange={(v) => setParam(node.id, p.name, v)}
                        />
                        {promoted && (
                          <div className="mt-0.5 text-[9px] text-muted-foreground">
                            已开放为输入（未连接时用此值）
                          </div>
                        )}
                      </>
                    )}
                  </div>
                );
              })
            )}
          </>
        )}

        {tab === "output" && node.data.stale && Object.keys(outputs).length > 0 && (
          <div className="mb-2 rounded border border-amber-500/40 bg-amber-500/10 px-2 py-1 text-[10px] text-amber-700 dark:text-amber-400">
            结果已过期：此节点或上游在上次运行后被修改，重新运行以更新。
          </div>
        )}
        {tab === "output" &&
          (Object.keys(outputs).length === 0 ? (
            <div className="text-muted-foreground">
              {node.data.status === "error" ? "执行失败，没有输出" : "尚未运行，暂无输出"}
            </div>
          ) : (
            // Render in declared (descriptor) order; outputs arrive as an unordered
            // map, so append any extra keys not in the descriptor at the end.
            [
              ...descriptor.outputs
                .filter((o) => outputs[o.name] !== undefined)
                .map((o) => [o.name, outputs[o.name]] as [string, PortValue]),
              ...Object.entries(outputs).filter(
                ([k]) => !descriptor.outputs.some((o) => o.name === k)
              ),
            ].map(([key, val]) => {
              const label = descriptor.outputs.find((o) => o.name === key)?.label ?? key;
              return (
                <div key={key} className="mb-3">
                  <div className="mb-1 flex items-center justify-between">
                    <span className="text-[11px] font-medium text-muted-foreground">
                      {label}
                    </span>
                    {val.type !== "image" && (
                      <CopyButton
                        text={val.type === "bytes" ? bytesToHex(val.value, Infinity) : valueText(val)}
                      />
                    )}
                  </div>
                  <OutputValue value={val} />
                </div>
              );
            })
          ))}

        {tab === "logs" &&
          (logs.length === 0 ? (
            <div className="text-muted-foreground">暂无日志</div>
          ) : (
            <div className="space-y-1">
              {logs.map((l, i) => (
                <div key={i} className="flex gap-2 font-mono text-[10px]">
                  <span className="text-muted-foreground">{l.time}</span>
                  <span
                    className={cn(
                      l.level === "error"
                        ? "text-destructive"
                        : l.level === "success"
                          ? "text-green-600"
                          : "text-muted-foreground"
                    )}
                  >
                    {l.message}
                  </span>
                </div>
              ))}
            </div>
          ))}
      </div>
    </div>
  );
}
