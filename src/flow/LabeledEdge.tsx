import {
  BaseEdge,
  EdgeLabelRenderer,
  getBezierPath,
  type EdgeProps,
} from "@xyflow/react";

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
  const descriptor = node
    ? useDescriptorStore.getState().byId[node.data.descriptorId]
    : undefined;
  const type = descriptor?.outputs.find((o) => o.name === sourceHandleId)?.type;

  return (
    <>
      <BaseEdge id={id} path={path} markerEnd={markerEnd} style={style} />
      {type && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              pointerEvents: "none",
            }}
            className="rounded border border-border bg-card px-1 text-[9px] font-medium text-muted-foreground shadow-sm"
          >
            {type}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}
