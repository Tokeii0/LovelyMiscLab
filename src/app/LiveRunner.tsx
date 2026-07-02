import { useEffect } from "react";

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
  const runRevision = useGraphStore((s) => s.runRevision);

  useEffect(() => {
    if (mode !== "live") return;
    const t = setTimeout(() => void executeGraph(), 120);
    return () => clearTimeout(t);
  }, [runRevision, mode]);

  return null;
}
