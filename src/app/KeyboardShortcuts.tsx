import { useEffect } from "react";

import { newFlow, openFlow, saveFlow } from "@/lib/project";
import { useCommandPaletteStore } from "@/store/commandPalette";
import { useGraphStore, type Clipboard } from "@/store/graph";

// Module-level clipboard (persists across renders; not reactive).
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

/** Global canvas keyboard shortcuts: copy/paste/cut/duplicate/select-all/delete. */
export function KeyboardShortcuts() {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // File shortcuts work everywhere (even while a field is focused).
      const fileMod = e.ctrlKey || e.metaKey;
      const fileKey = e.key.toLowerCase();
      if (fileMod && fileKey === "k") {
        e.preventDefault();
        useCommandPaletteStore.getState().toggle();
        return;
      }
      if (fileMod && (fileKey === "s" || fileKey === "o" || fileKey === "n")) {
        e.preventDefault();
        if (fileKey === "s") void saveFlow();
        else if (fileKey === "o") void openFlow();
        else newFlow();
        return;
      }

      // Don't hijack typing in inputs/textareas/selects (node inline fields, inspector).
      const t = e.target as HTMLElement | null;
      if (
        t &&
        (t.tagName === "INPUT" ||
          t.tagName === "TEXTAREA" ||
          t.tagName === "SELECT" ||
          t.isContentEditable)
      ) {
        return;
      }

      const g = useGraphStore.getState();
      const mod = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();

      if (mod && key === "c") {
        const clip = buildClip();
        if (clip) {
          clipboard = clip;
          pasteOffset = 0;
          e.preventDefault();
        }
      } else if (mod && key === "z") {
        if (e.shiftKey) g.redo();
        else g.undo();
        e.preventDefault();
      } else if (mod && key === "y") {
        g.redo();
        e.preventDefault();
      } else if (mod && key === "v") {
        if (clipboard) {
          pasteOffset += 32;
          g.paste(clipboard, pasteOffset, pasteOffset);
          e.preventDefault();
        }
      } else if (mod && key === "x") {
        const clip = buildClip();
        if (clip) {
          clipboard = clip;
          pasteOffset = 0;
          g.nodes.filter((n) => n.selected).forEach((n) => g.deleteNode(n.id));
          e.preventDefault();
        }
      } else if (mod && key === "d") {
        const clip = buildClip();
        if (clip) {
          g.paste(clip, 32, 32);
          e.preventDefault();
        }
      } else if (mod && key === "a") {
        g.selectAll();
        e.preventDefault();
      } else if (key === "delete" || key === "backspace") {
        const nodes = g.nodes.filter((n) => n.selected);
        const dges = g.edges.filter((ed) => ed.selected);
        if (nodes.length || dges.length) {
          nodes.forEach((n) => g.deleteNode(n.id));
          dges.forEach((ed) => g.deleteEdge(ed.id));
          e.preventDefault();
        }
      } else if (key === "escape") {
        g.deselectAll();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return null;
}
