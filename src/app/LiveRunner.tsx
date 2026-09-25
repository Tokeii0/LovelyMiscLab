import { useEffect } from "react";

import { requestRun } from "@/flow/runner";
import { useAgentStore } from "@/store/agent";
import { useGraphStore } from "@/store/graph";
import { useRunStore } from "@/store/run";

/**
 * In live mode, re-run (incrementally) shortly after the graph's structure or
 * params change. runRevision ignores run results and selection, so a run's own
 * updates never retrigger it.
 */
export function LiveRunner() {
  const mode = useRunStore((s) => s.mode);
  const runRevision = useGraphStore((s) => s.runRevision);

  useEffect(() => {
    // An AI build changes the graph every step; it runs the result once at the end.
    if (mode !== "live" || useAgentStore.getState().running) return;
    const t = setTimeout(() => void requestRun({ kind: "live" }), 150);
    return () => clearTimeout(t);
  }, [runRevision, mode]);

  return null;
}
