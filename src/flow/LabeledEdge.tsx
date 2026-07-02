import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from "@xyflow/react";

import type { PortValue } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";

/** Bezier edge that labels itself with the source port's data type. */
export function LabeledEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  source,
  sourceHandleId,
  markerEnd,
  style,
}: EdgeProps) {
  const [path, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const node = useGraphStore.getState().nodes.find((n) => n.id === source);
  const sourceValue = useGraphStore((s) =>
    s.nodes.find((n) => n.id === source)?.data.outputs?.[sourceHandleId ?? ""]
  );
  const descriptor = node
    ? useDescriptorStore.getState().byId[node.data.descriptorId]
    : undefined;
  const type = descriptor?.outputs.find((o) => o.name === sourceHandleId)?.type;
  const preview = sourceValue ? shortValue(sourceValue) : "";

  return (
    <>
      <BaseEdge id={id} path={path} markerEnd={markerEnd} style={style} />
      {(type || preview) && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              pointerEvents: "none",
            }}
            title={preview}
            className="max-w-32 truncate rounded border border-border bg-card px-1 text-[9px] font-medium text-muted-foreground shadow-sm"
          >
            {preview ? `${type ?? "value"} · ${preview}` : type}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}

function shortValue(v: PortValue): string {
  switch (v.type) {
    case "text":
      return v.value.slice(0, 36);
    case "number":
      return String(v.value);
    case "bool":
      return v.value ? "true" : "false";
    case "stringList":
      return `${v.value.length} 项`;
    case "candidates":
      return `${v.value.length} 候选`;
    case "bytes":
      return `${v.value.length} bytes`;
    case "image":
      return "image";
    case "json":
    case "fingerprint":
      return JSON.stringify(v.value).slice(0, 36);
    case "artifact":
      return v.value;
    default:
      return "";
  }
}
