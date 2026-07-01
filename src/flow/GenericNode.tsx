import { memo } from "react";
import { Handle, NodeToolbar, Position, type NodeProps } from "@xyflow/react";
import { Ban, Check, Copy, Eye, Play, Trash2, X } from "lucide-react";

import type { PortValue } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore, type FlowNodeData } from "@/store/graph";
import { useInspectorStore } from "@/store/inspector";

import { nodeIcon } from "./nodeIcons";
import { portColor } from "./portColors";
import { runSingleNode } from "./runner";

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
  const setInspectorTab = useInspectorStore((s) => s.setTab);

  if (!descriptor) {
    return (
      <div className="rounded-lg border border-destructive bg-card p-2 text-xs text-destructive">
        未知模块: {data.descriptorId}
      </div>
    );
  }

  const Icon = nodeIcon(descriptor.id, descriptor.category);
  const outputs = data.outputs;
  const firstOut = outputs ? Object.entries(outputs)[0] : undefined;

  const action =
    "flex h-6 w-6 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-accent hover:text-foreground";

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
          <span className="flex-1 truncate text-xs font-medium">{descriptor.displayName}</span>
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
          <div className="h-0.5 w-full bg-secondary">
            <div
              className="h-full bg-primary transition-all"
              style={{ width: `${Math.round((data.progress ?? 0) * 100)}%` }}
            />
          </div>
        )}

        <div className="flex flex-col gap-1 p-2">
          {descriptor.inputs.map((p) => (
            <div key={`in-${p.name}`} className="flex items-center gap-1.5 text-[11px]">
              <Handle
                type="target"
                position={Position.Left}
                id={p.name}
                style={handleStyle(portColor(p.type))}
              />
              <span className="text-muted-foreground">{p.label}</span>
            </div>
          ))}
          {descriptor.outputs.map((p) => (
            <div
              key={`out-${p.name}`}
              className="flex items-center justify-end gap-1.5 text-[11px]"
            >
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

        {firstOut &&
          (firstOut[1].type === "image" ? (
            <div className="border-t border-border bg-secondary/40 p-1">
              <img
                src={firstOut[1].value}
                alt=""
                className="max-h-32 w-full rounded object-contain"
              />
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
