import { useState } from "react";
import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  useInternalNode,
  type EdgeProps,
} from "@xyflow/react";

import type { PortValue } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import type { FlowNodeData } from "@/store/graph";
import { usePrefs } from "@/store/prefs";

import { portTypeLabel } from "./portColors";

/** Bezier edge labelled with the data type (and a value preview) it carries. By
 * default the label shows only on hover or when the edge or an end is selected,
 * so busy graphs stay readable. */
export function LabeledEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  source,
  target,
  sourceHandleId,
  markerEnd,
  style,
  selected,
}: EdgeProps) {
  const [path, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });
  const [hover, setHover] = useState(false);
  const always = usePrefs((s) => s.edgeLabels === "always");

  // O(1) lookups (a find over all nodes per edge made every update O(E×N)).
  const src = useInternalNode(source);
  const dst = useInternalNode(target);
  const data = src?.data as FlowNodeData | undefined;
  const descriptor = useDescriptorStore((s) => (data ? s.byId[data.descriptorId] : undefined));
  const sourceValue = data?.outputs?.[sourceHandleId ?? ""];
  const type = descriptor?.outputs.find((o) => o.name === sourceHandleId)?.type;
  const preview = sourceValue ? shortValue(sourceValue) : "";
  const show = always || hover || selected || src?.selected || dst?.selected;

  return (
    <g onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}>
      <BaseEdge id={id} path={path} markerEnd={markerEnd} style={style} interactionWidth={16} />
      {show && (type || preview) && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              pointerEvents: "none",
            }}
            title={preview}
            className="max-w-40 truncate rounded border border-border bg-card px-1 text-[9px] font-medium text-muted-foreground shadow-sm"
          >
            {type ? portTypeLabel(type) : "值"}
            {preview ? ` · ${preview}` : ""}
          </div>
        </EdgeLabelRenderer>
      )}
    </g>
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
      return `${v.value.length} 字节`;
    case "image":
      return "图片";
    case "json":
    case "fingerprint":
      return JSON.stringify(v.value).slice(0, 36);
    case "artifact":
      return v.value;
    default:
      return "";
  }
}
