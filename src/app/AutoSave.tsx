import { useEffect, useMemo } from "react";

import { saveAutoDraft } from "@/lib/project";
import { useGraphStore } from "@/store/graph";
import { useProjectStore } from "@/store/project";

/** Persist a lightweight recovery draft whenever the editable graph changes. */
export function AutoSave() {
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const projectName = useProjectStore((s) => s.name);
  const projectPath = useProjectStore((s) => s.path);

  const signature = useMemo(
    () =>
      JSON.stringify([
        projectName,
        projectPath,
        nodes.map((n) => [
          n.id,
          n.data.descriptorId,
          n.data.label,
          n.data.color,
          n.data.params,
          n.data.inputParams,
          n.data.disabled,
          n.position.x,
          n.position.y,
        ]),
        edges.map((e) => [e.id, e.source, e.sourceHandle, e.target, e.targetHandle, e.type]),
      ]),
    [edges, nodes, projectName, projectPath]
  );

  useEffect(() => {
    if (nodes.length === 0 && edges.length === 0) return;
    const t = window.setTimeout(saveAutoDraft, 600);
    return () => window.clearTimeout(t);
  }, [signature, nodes.length, edges.length]);

  return null;
}
