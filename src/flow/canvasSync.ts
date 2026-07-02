// Bidirectional live-sync between the frontend React Flow store and the backend
// canvas mirror, so the embedded MCP server can read (and, in M5, modify) the
// canvas the user is looking at.
//
//  • Frontend → backend: debounced `sync_canvas` on every store change.
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

function snapshot(): CanvasSnapshot {
  return { ...graphToSaved(), rev: ++rev };
}

function pushNow() {
  if (!inTauri || applyingRemote) return;
  api.syncCanvas(snapshot()).catch((e) => console.error("syncCanvas failed", e));
}

function schedulePush() {
  if (applyingRemote) return;
  clearTimeout(timer);
  timer = window.setTimeout(pushNow, 250);
}

/** Wire up both directions. Returns a cleanup fn. No-op outside Tauri. */
export function startCanvasSync(): () => void {
  if (!inTauri) return () => {};

  // Seed the backend mirror immediately so `get_canvas` is populated.
  pushNow();

  const unsub = useGraphStore.subscribe((s, prev) => {
    if (s.nodes !== prev.nodes || s.edges !== prev.edges) schedulePush();
  });

  // AI-applied canvas updates land here. `applyingRemote` suppresses the echo
  // push that `loadFlow`'s store mutation would otherwise trigger.
  const unlisten = listen<CanvasSnapshot>("mcp://canvas-update", (e) => {
    applyingRemote = true;
    try {
      rev = Math.max(rev, e.payload.rev);
      useGraphStore.getState().loadFlow(e.payload.nodes, e.payload.edges);
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
