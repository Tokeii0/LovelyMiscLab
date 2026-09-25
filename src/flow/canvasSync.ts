// Bidirectional live-sync between the frontend React Flow store and the backend
// canvas mirror, so the embedded MCP server can read and modify the canvas the
// user is looking at.
//
//  • Frontend → backend: debounced `sync_canvas` after *edits* (editRevision),
//    and only while the MCP server runs — run progress, selection and drags in
//    flight don't re-send the whole graph over IPC.
//  • Backend → frontend: `mcp://canvas-update` events applied via `loadFlow`.
//
// The echo loop (push → emit → loadFlow → store change → push …) is broken by
// the `applyingRemote` flag (primary) plus a monotonic `rev` (backstop).

import { listen } from "@tauri-apps/api/event";

import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { graphToSaved, type SavedEdge, type SavedNode } from "@/lib/project";
import { useGraphStore } from "@/store/graph";

export interface CanvasSnapshot {
  nodes: SavedNode[];
  edges: SavedEdge[];
  rev: number;
}

let rev = 0;
let applyingRemote = false;
let timer: number | undefined;
/** True while the embedded MCP server runs (nobody reads the mirror otherwise). */
let active = false;

/** Called with the MCP server's state; starting it pushes the current canvas. */
export function setCanvasSyncActive(on: boolean) {
  const turnedOn = on && !active;
  active = on;
  if (turnedOn) pushNow();
}

function snapshot(): CanvasSnapshot {
  return { ...graphToSaved(), rev: ++rev };
}

function pushNow() {
  if (!inTauri || applyingRemote || !active) return;
  api.syncCanvas(snapshot()).catch((e) => console.error("syncCanvas failed", e));
}

function schedulePush() {
  if (applyingRemote || !active) return;
  clearTimeout(timer);
  timer = window.setTimeout(pushNow, 250);
}

/** Wire up both directions. Returns a cleanup fn. No-op outside Tauri. */
export function startCanvasSync(): () => void {
  if (!inTauri) return () => {};

  // The server may already be running (auto-start); seed the mirror if so.
  api
    .mcpStatus()
    .then((st) => setCanvasSyncActive(st.running))
    .catch(() => {});

  const unsub = useGraphStore.subscribe((s, prev) => {
    if (s.editRevision !== prev.editRevision) schedulePush();
  });

  // AI-applied canvas updates land here. `applyingRemote` suppresses the echo
  // push that `loadFlow`'s store mutation would otherwise trigger.
  const unlisten = listen<CanvasSnapshot>("mcp://canvas-update", (e) => {
    applyingRemote = true;
    try {
      rev = Math.max(rev, e.payload.rev);
      // Recorded as one undo step, keeping results of nodes the AI didn't touch.
      useGraphStore
        .getState()
        .loadFlow(e.payload.nodes, e.payload.edges, { history: "record", keepRuntime: true });
    } finally {
      applyingRemote = false;
    }
  });

  return () => {
    clearTimeout(timer);
    unsub();
    unlisten.then((un) => un());
  };
}
