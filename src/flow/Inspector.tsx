import { useState } from "react";
import { Check, Copy, Link2, Play, StepForward } from "lucide-react";

import { cn } from "@/lib/utils";
import type { PortValue } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore, type FlowNode } from "@/store/graph";
import { useInspectorStore, type InspectorTab as Tab } from "@/store/inspector";

import { nodeIcon } from "./nodeIcons";
import { executeToNode, runSingleNode } from "./runner";
import { WidgetRenderer } from "./WidgetRenderer";

function valueText(v: PortValue): string {
  switch (v.type) {
    case "text":
      return v.value;
    case "number":
      return String(v.value);
    case "bool":
      return v.value ? "true" : "false";
    case "stringList":
      return v.value.join("\n");
    case "candidates":
      return v.value.map((c) => `${c.score.toFixed(2)}  ${c.text}`).join("\n");
    case "bytes":
      return `<${v.value.length} 字节>`;
    case "json":
    case "fingerprint":
      return JSON.stringify(v.value, null, 2);
    default:
      return "";
  }
}

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
          <input
            value={node.data.label || descriptor.displayName}
            onChange={(e) => renameNode(node.id, e.target.value)}
            placeholder={descriptor.displayName}
            title="节点名称（可修改）"
            className="min-w-0 flex-1 rounded border border-transparent bg-transparent text-sm font-semibold hover:border-border focus:border-input focus:bg-background focus:outline-none"
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
            onClick={() => void runSingleNode(node.id)}
            className="flex items-center gap-1 rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            <Play className="h-3 w-3" />
            单独执行
          </button>
          <button
            onClick={() => void executeToNode(node.id)}
            className="flex items-center gap-1 rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            <StepForward className="h-3 w-3" />
            运行到此处
          </button>
        </div>
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

        {tab === "output" &&
          (Object.keys(outputs).length === 0 ? (
            <div className="text-muted-foreground">尚未运行，暂无输出</div>
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
                    {val.type !== "image" && <CopyButton text={valueText(val)} />}
                  </div>
                  {val.type === "image" ? (
                    <img
                      src={val.value}
                      alt={label}
                      className="max-h-64 w-full rounded border border-border bg-white object-contain"
                    />
                  ) : (
                    <pre className="max-h-64 select-text overflow-auto whitespace-pre-wrap break-all rounded bg-background p-2 font-mono text-[10px] leading-snug">
                      {valueText(val) || "（空）"}
                    </pre>
                  )}
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
