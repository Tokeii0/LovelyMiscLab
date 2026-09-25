import { useEffect } from "react";

import { installCloseGuard, offerDraftRestore, saveAutoDraft } from "@/lib/project";
import { useGraphStore } from "@/store/graph";
import { useProjectStore } from "@/store/project";

/** Keeps a crash-recovery draft of the canvas, offers it back on startup, and
 * asks before the window closes with unsaved edits. */
export function AutoSave() {
  // editRevision moves on every user edit (never on run results or selection),
  // so this doesn't re-serialize the graph on each progress event or drag frame.
  const editRevision = useGraphStore((s) => s.editRevision);
  const savedRevision = useProjectStore((s) => s.savedRevision);
  const name = useProjectStore((s) => s.name);
  const path = useProjectStore((s) => s.path);

  useEffect(() => {
    offerDraftRestore();
    return installCloseGuard();
  }, []);

  useEffect(() => {
    if (useGraphStore.getState().nodes.length === 0) return;
    const t = window.setTimeout(saveAutoDraft, 600);
    return () => window.clearTimeout(t);
  }, [editRevision, savedRevision, name, path]);

  return null;
}
