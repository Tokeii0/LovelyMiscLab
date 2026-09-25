// In-app node clipboard: copy / cut / paste / duplicate of the canvas selection.

import { useGraphStore, type Clipboard } from "@/store/graph";

let clipboard: Clipboard | null = null;
let pasteOffset = 0;

/** Snapshot the currently-selected nodes + the edges wholly between them. */
function buildClip(): Clipboard | null {
  const g = useGraphStore.getState();
  const sel = g.nodes.filter((n) => n.selected);
  if (sel.length === 0) return null;
  const ids = new Set(sel.map((n) => n.id));
  return {
    nodes: sel.map((n) => ({
      oldId: n.id,
      descriptorId: n.data.descriptorId,
      label: n.data.label,
      color: n.data.color,
      params: { ...n.data.params },
      inputParams: [...(n.data.inputParams ?? [])],
      position: { ...n.position },
    })),
    edges: g.edges
      .filter((e) => ids.has(e.source) && ids.has(e.target))
      .map((e) => ({
        source: e.source,
        sourceHandle: e.sourceHandle,
        target: e.target,
        targetHandle: e.targetHandle,
      })),
  };
}

export function hasClipboard(): boolean {
  return clipboard !== null;
}

/** Returns false when nothing is selected. */
export function copySelection(): boolean {
  const clip = buildClip();
  if (!clip) return false;
  clipboard = clip;
  pasteOffset = 0;
  return true;
}

export function cutSelection(): boolean {
  if (!copySelection()) return false;
  useGraphStore.getState().deleteSelection();
  return true;
}

/**
 * Paste the clipboard. With `at` (flow coordinates) the pasted group's top-left
 * lands there; otherwise each paste steps 32px down-right from the originals.
 */
export function pasteClipboard(at?: { x: number; y: number }): boolean {
  if (!clipboard || clipboard.nodes.length === 0) return false;
  let dx: number;
  let dy: number;
  if (at) {
    const minX = Math.min(...clipboard.nodes.map((n) => n.position.x));
    const minY = Math.min(...clipboard.nodes.map((n) => n.position.y));
    dx = at.x - minX;
    dy = at.y - minY;
  } else {
    pasteOffset += 32;
    dx = dy = pasteOffset;
  }
  useGraphStore.getState().paste(clipboard, dx, dy);
  return true;
}

export function duplicateSelection(): boolean {
  const clip = buildClip();
  if (!clip) return false;
  useGraphStore.getState().paste(clip, 32, 32);
  return true;
}
