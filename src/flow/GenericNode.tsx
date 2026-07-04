import { memo, useState } from "react";
import { Handle, NodeToolbar, Position, type NodeProps } from "@xyflow/react";
import { Ban, Check, Copy, Eye, Play, Sparkles, Trash2, X } from "lucide-react";

import { ProgressBar } from "@/components/ui/progress";
import type { PortValue } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore, type FlowNodeData } from "@/store/graph";
import { useImageViewer } from "@/store/imageViewer";
import { useInspectorStore } from "@/store/inspector";
import { usePortSuggest } from "@/store/portSuggest";

import { nodeIcon } from "./nodeIcons";
import { paramPortType, portColor } from "./portColors";
import { runSingleNode } from "./runner";
import { WidgetRenderer } from "./WidgetRenderer";

// In-flow handle — each port sits on its own row; React Flow still measures it.
const handleStyle = (color: string): React.CSSProperties => ({
  position: "relative",
  transform: "none",
  left: "auto",
  right: "auto",
  top: "auto",
  width: 9,
  height: 9,
  borderRadius: 9999,
  background: color,
  border: "2px solid var(--card)",
});

function shortText(v: PortValue): string {
  switch (v.type) {
    case "text":
      return v.value;
    case "number":
      return String(v.value);
    case "bool":
      return v.value ? "true" : "false";
    case "stringList":
      return `${v.value.length} 项`;
    case "candidates":
      return `${v.value.length} 候选`;
    case "bytes":
      return `${v.value.length} 字节`;
    case "json":
    case "fingerprint":
      return JSON.stringify(v.value);
    default:
      return "";
  }
}

/** Hover-revealed ✨ button on a port row → opens the AI next-node suggester.
 * Absolutely positioned in the row's empty inner gutter so it never overlaps the
 * connection Handle or shifts layout; `nodrag` keeps it from starting a drag. */
function PortSparkle({
  side,
  onClick,
}: {
  side: "left" | "right";
  onClick: (e: React.MouseEvent) => void;
}) {
  return (
    <button
      type="button"
      title="AI 推荐下一个节点"
      onClick={onClick}
      className={cn(
        "nodrag absolute top-1/2 z-10 flex h-4 w-4 -translate-y-1/2 items-center justify-center rounded",
        "bg-card text-primary opacity-0 shadow-sm ring-1 ring-primary/30 transition-opacity",
        "hover:bg-primary/10 group-hover/port:opacity-100",
        side === "left" ? "left-1" : "right-1"
      )}
    >
      <Sparkles className="h-2.5 w-2.5" />
    </button>
  );
}

function StatusIcon({ status }: { status: FlowNodeData["status"] }) {
  if (status === "done") return <Check className="h-3.5 w-3.5 text-green-600" />;
  if (status === "error") return <X className="h-3.5 w-3.5 text-destructive" />;
  if (status === "running")
    return <span className="h-2 w-2 animate-pulse rounded-full bg-blue-500" />;
  return <span className="h-2 w-2 rounded-full bg-muted-foreground/40" />;
}

function GenericNodeImpl({ id, data: raw, selected }: NodeProps) {
  const data = raw as FlowNodeData;
  const descriptor = useDescriptorStore((s) => s.byId[data.descriptorId]);
  const setSelected = useGraphStore((s) => s.setSelected);
  const deleteNode = useGraphStore((s) => s.deleteNode);
  const duplicateNode = useGraphStore((s) => s.duplicateNode);
  const setDisabled = useGraphStore((s) => s.setDisabled);
  const setParam = useGraphStore((s) => s.setParam);
  const renameNode = useGraphStore((s) => s.renameNode);
  const setInspectorTab = useInspectorStore((s) => s.setTab);
  const [editing, setEditing] = useState(false);

  if (!descriptor) {
    return (
      <div className="rounded-lg border border-destructive bg-card p-2 text-xs text-destructive">
        未知模块: {data.descriptorId}
      </div>
    );
  }

  const Icon = nodeIcon(descriptor.id, descriptor.category);
  // Latest log line, shown under the progress bar as the current stage.
  const runningStatus = data.logs?.[data.logs.length - 1]?.message;
  // Dedicated viewer nodes get a richer on-node preview: a larger image / a
  // multi-line scrollable text block instead of the default one-line preview.
  const isImageView = descriptor.id === "image_view";
  const isTextView = descriptor.id === "text_view";
  const outputs = data.outputs;
  // Preview the first *declared* output that has a value. Node outputs arrive as
  // an unordered map (Rust HashMap → JSON), so iterating by key order would pick
  // a random port each run; follow the descriptor's declared order instead.
  const firstOut: [string, PortValue] | undefined = (() => {
    if (!outputs) return undefined;
    for (const p of descriptor.outputs) {
      const v = outputs[p.name];
      if (v !== undefined) return [p.name, v];
    }
    return Object.entries(outputs)[0];
  })();
  // Source nodes (no input ports) show their param widgets inline for direct entry.
  const inlineParams =
    descriptor.inputs.length === 0
      ? descriptor.params.filter((p) => !(data.inputParams ?? []).includes(p.name))
      : [];

  const action =
    "flex h-6 w-6 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-accent hover:text-foreground";

  const openSuggest = (e: React.MouseEvent, port: string, dir: "in" | "out") => {
    e.stopPropagation();
    usePortSuggest.getState().open({
      nodeId: id,
      descriptorId: descriptor.id,
      port,
      dir,
      anchor: { x: e.clientX, y: e.clientY },
    });
  };

  return (
    <>
      <NodeToolbar
        isVisible={selected}
        position={Position.Top}
        offset={8}
        className="flex items-center gap-0.5 rounded-lg border border-border bg-card p-0.5 shadow-md"
      >
        <button className={action} title="执行" onClick={() => void runSingleNode(id)}>
          <Play className="h-3.5 w-3.5" />
        </button>
        <button className={action} title="复制" onClick={() => duplicateNode(id)}>
          <Copy className="h-3.5 w-3.5" />
        </button>
        <button
          className={action}
          title={data.disabled ? "启用" : "禁用"}
          onClick={() => setDisabled(id, !data.disabled)}
        >
          <Ban className="h-3.5 w-3.5" />
        </button>
        <button
          className={action}
          title="查看输出"
          onClick={() => {
            setSelected(id);
            setInspectorTab("output");
          }}
        >
          <Eye className="h-3.5 w-3.5" />
        </button>
        <button
          className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
          title="删除"
          onClick={() => deleteNode(id)}
        >
          <Trash2 className="h-3.5 w-3.5" />
        </button>
      </NodeToolbar>

      <div
        className={cn(
          "w-[200px] overflow-hidden rounded-lg border bg-card shadow-sm transition-all",
          data.disabled && "opacity-50",
          data.status === "error"
            ? "border-destructive"
            : selected
              ? "border-primary ring-2 ring-primary/25"
              : "border-border"
        )}
      >
        <div className="flex items-center gap-1.5 border-b border-border px-2 py-1.5">
          <span
            className="flex h-5 w-5 shrink-0 items-center justify-center rounded"
            style={{ background: `${descriptor.color}18`, color: descriptor.color }}
          >
            <Icon className="h-3.5 w-3.5" />
          </span>
          {editing ? (
            <input
              autoFocus
              defaultValue={data.label || descriptor.displayName}
              onClick={(e) => e.stopPropagation()}
              onBlur={(e) => {
                renameNode(id, e.target.value);
                setEditing(false);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  renameNode(id, (e.target as HTMLInputElement).value);
                  setEditing(false);
                } else if (e.key === "Escape") {
                  setEditing(false);
                }
              }}
              className="nodrag min-w-0 flex-1 rounded border border-input bg-background px-1 text-xs font-medium focus:outline-none"
            />
          ) : (
            <span
              className="flex-1 truncate text-xs font-medium"
              title="双击重命名"
              onDoubleClick={(e) => {
                e.stopPropagation();
                setEditing(true);
              }}
            >
              {data.label?.trim() || descriptor.displayName}
            </span>
          )}
          <StatusIcon status={data.status} />
          <button
            className="rounded p-0.5 text-muted-foreground transition-colors hover:bg-accent hover:text-primary"
            title="单独执行"
            onClick={(e) => {
              e.stopPropagation();
              void runSingleNode(id);
            }}
          >
            <Play className="h-3 w-3" />
          </button>
        </div>

        {data.status === "running" && (
          <div className="border-b border-border px-2 py-1.5">
            <ProgressBar value={data.progress ?? 0} status={runningStatus} />
          </div>
        )}

        <div className="flex flex-col gap-1 p-2">
          {descriptor.inputs.map((p) => (
            <div
              key={`in-${p.name}`}
              className="group/port relative flex items-center gap-1.5 text-[11px]"
            >
              <Handle
                type="target"
                position={Position.Left}
                id={p.name}
                style={handleStyle(portColor(p.type))}
              />
              <span className="text-muted-foreground">{p.label}</span>
              <PortSparkle side="right" onClick={(e) => openSuggest(e, p.name, "in")} />
            </div>
          ))}
          {(data.inputParams ?? []).map((name) => {
            const spec = descriptor.params.find((pp) => pp.name === name);
            if (!spec) return null;
            return (
              <div key={`param-${name}`} className="flex items-center gap-1.5 text-[11px]">
                <Handle
                  type="target"
                  position={Position.Left}
                  id={name}
                  style={handleStyle(portColor(paramPortType(spec.widget)))}
                />
                <span className="italic text-muted-foreground/80">{spec.label}</span>
              </div>
            );
          })}
          {descriptor.outputs.map((p) => (
            <div
              key={`out-${p.name}`}
              className="group/port relative flex items-center justify-end gap-1.5 text-[11px]"
            >
              <PortSparkle side="left" onClick={(e) => openSuggest(e, p.name, "out")} />
              <span className="text-muted-foreground">{p.label}</span>
              <Handle
                type="source"
                position={Position.Right}
                id={p.name}
                style={handleStyle(portColor(p.type))}
              />
            </div>
          ))}
        </div>

        {inlineParams.length > 0 && (
          <div className="flex flex-col gap-1.5 border-t border-border p-2">
            {inlineParams.map((p) => (
              <div key={`ip-${p.name}`} className="text-[10px]">
                {inlineParams.length > 1 && (
                  <div className="mb-0.5 text-muted-foreground">{p.label}</div>
                )}
                <WidgetRenderer
                  spec={p}
                  value={data.params[p.name]}
                  onChange={(v) => setParam(id, p.name, v)}
                />
              </div>
            ))}
          </div>
        )}

        {firstOut &&
          (firstOut[1].type === "image" ? (
            <div className="border-t border-border bg-secondary/40 p-1">
              <img
                src={firstOut[1].value}
                alt=""
                title="点击查看大图 / 调整"
                onClick={(e) => {
                  e.stopPropagation();
                  useImageViewer
                    .getState()
                    .show(
                      (firstOut[1] as Extract<PortValue, { type: "image" }>).value,
                      data.label?.trim() || descriptor.displayName
                    );
                }}
                className={cn(
                  "nodrag w-full cursor-zoom-in rounded object-contain",
                  isImageView ? "max-h-72" : "max-h-32"
                )}
              />
            </div>
          ) : isTextView ? (
            <div className="border-t border-border bg-secondary/40 p-1">
              <pre className="nodrag nowheel max-h-44 overflow-auto whitespace-pre-wrap break-all rounded bg-background/60 px-2 py-1 font-mono text-[10px] leading-snug">
                {shortText(firstOut[1]).slice(0, 4000) || "（空）"}
              </pre>
            </div>
          ) : (
            <div
              className="truncate border-t border-border bg-secondary/40 px-2 py-1 font-mono text-[10px]"
              title={shortText(firstOut[1])}
            >
              {shortText(firstOut[1]) || "（空）"}
            </div>
          ))}

        {data.status === "error" && data.error && (
          <div className="border-t border-destructive/30 bg-destructive/10 px-2 py-1 text-[10px] text-destructive">
            {data.error}
          </div>
        )}
      </div>
    </>
  );
}

export const GenericNode = memo(GenericNodeImpl);
