import { memo, useEffect } from "react";
import { Handle, NodeToolbar, Position, type NodeProps } from "@xyflow/react";
import { Play, Trash2 } from "lucide-react";

import { cn } from "@/lib/utils";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore, type FlowNodeData } from "@/store/graph";

import { nodeIcon } from "./nodeIcons";
import { portColor } from "./portColors";
import { runSingleNode } from "./runner";

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

/**
 * A value picker. It reads the option list of whatever *select* parameter its
 * `value` output is wired into (e.g. 哈希计算 的「算法」) and renders those as a
 * dropdown, so you pick a validated value that drives the connected parameter.
 * With no (or a non-select) connection it falls back to a free-text field.
 */
function SelectorNodeImpl({ id, data: raw, selected }: NodeProps) {
  const data = raw as FlowNodeData;
  const descriptor = useDescriptorStore((s) => s.byId[data.descriptorId]);
  const byId = useDescriptorStore((s) => s.byId);
  const setParam = useGraphStore((s) => s.setParam);
  const deleteNode = useGraphStore((s) => s.deleteNode);
  const edges = useGraphStore((s) => s.edges);
  const nodes = useGraphStore((s) => s.nodes);

  const value = (data.params.value as string) ?? "";

  // Pull the option list from the connected target's select parameter.
  let options: string[] | null = null;
  let targetLabel = "";
  const outEdge = edges.find((e) => e.source === id && e.sourceHandle === "value");
  if (outEdge) {
    const target = nodes.find((n) => n.id === outEdge.target);
    const tdesc = target ? byId[target.data.descriptorId] : undefined;
    const param = tdesc?.params.find((p) => p.name === outEdge.targetHandle);
    if (param && param.widget.kind === "select") {
      options = param.widget.options;
      targetLabel = `${tdesc?.displayName ?? ""} · ${param.label}`;
    }
  }

  // When a fresh option list is pulled and the current value isn't in it, adopt
  // the first option so the connected parameter gets a valid value immediately.
  useEffect(() => {
    if (options && options.length > 0 && !options.includes(value)) {
      setParam(id, "value", options[0], { history: false });
    }
  }, [options, value, id, setParam]);

  if (!descriptor) {
    return (
      <div className="rounded-lg border border-destructive bg-card p-2 text-xs text-destructive">
        未知模块: {data.descriptorId}
      </div>
    );
  }
  const Icon = nodeIcon(descriptor.id, descriptor.category);
  const btn =
    "flex h-6 w-6 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-accent hover:text-foreground";

  return (
    <>
      <NodeToolbar
        isVisible={selected}
        position={Position.Top}
        offset={8}
        className="flex items-center gap-0.5 rounded-lg border border-border bg-card p-0.5 shadow-md"
      >
        <button className={btn} title="执行" onClick={() => void runSingleNode(id)}>
          <Play className="h-3.5 w-3.5" />
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
          "w-[200px] overflow-hidden rounded-lg border bg-card shadow-sm",
          selected ? "border-primary ring-2 ring-primary/25" : "border-border"
        )}
      >
        <div className="flex items-center gap-1.5 border-b border-border px-2 py-1.5">
          <span
            className="flex h-5 w-5 shrink-0 items-center justify-center rounded"
            style={{ background: `${descriptor.color}18`, color: descriptor.color }}
          >
            <Icon className="h-3.5 w-3.5" />
          </span>
          <span className="flex-1 truncate text-xs font-medium">
            {data.label?.trim() || descriptor.displayName}
          </span>
        </div>

        <div className="flex flex-col gap-1.5 p-2">
          {options ? (
            <>
              <div className="truncate text-[10px] text-muted-foreground" title={targetLabel}>
                → {targetLabel}
              </div>
              <select
                className="nodrag w-full rounded border border-input bg-background px-1.5 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-primary"
                value={options.includes(value) ? value : (options[0] ?? "")}
                onChange={(e) => setParam(id, "value", e.target.value)}
              >
                {options.map((o) => (
                  <option key={o} value={o}>
                    {o}
                  </option>
                ))}
              </select>
            </>
          ) : (
            <>
              <input
                className="nodrag w-full rounded border border-input bg-background px-1.5 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-primary"
                value={value}
                placeholder="值"
                onChange={(e) => setParam(id, "value", e.target.value)}
              />
              <div className="text-[10px] leading-snug text-muted-foreground">
                连接到某节点的「选择」参数以拉取选项
              </div>
            </>
          )}
          <div className="flex items-center justify-end gap-1.5 text-[11px]">
            <span className="text-muted-foreground">值</span>
            <Handle
              type="source"
              position={Position.Right}
              id="value"
              style={handleStyle(portColor("text"))}
            />
          </div>
        </div>
      </div>
    </>
  );
}

export const SelectorNode = memo(SelectorNodeImpl);
