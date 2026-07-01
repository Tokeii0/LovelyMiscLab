import { useEffect, useMemo } from "react";

import { executeGraph } from "@/flow/runner";
import { useGraphStore } from "@/store/graph";
import { useRunStore } from "@/store/run";

/**
 * In live mode, re-run (incrementally) whenever the graph structure or params
 * change. The signature intentionally excludes runtime fields (status/outputs)
 * so a run's own updates don't retrigger it.
 */
export function LiveRunner() {
  const mode = useRunStore((s) => s.mode);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);

  const signature = useMemo(
    () =>
      JSON.stringify([
        nodes.map((n) => [n.id, n.data.descriptorId, n.data.params]),
        edges.map((e) => [e.source, e.sourceHandle, e.target, e.targetHandle]),
      ]),
    [nodes, edges]
  );

  useEffect(() => {
    if (mode !== "live") return;
    const t = setTimeout(() => void executeGraph(), 120);
    return () => clearTimeout(t);
  }, [signature, mode]);

  return null;
}
