import { useEffect } from "react";

import { copySelection, cutSelection, duplicateSelection, pasteClipboard } from "@/flow/clipboard";
import { runGraph } from "@/flow/runner";
import { newFlow, openFlow, saveFlow, saveFlowAs } from "@/lib/project";
import { hasTextSelection, isTextInput } from "@/lib/shortcuts";
import { useCommandPaletteStore } from "@/store/commandPalette";
import { useGraphStore } from "@/store/graph";
import { isAnyModalOpen } from "@/store/modal";
import { useViewStore } from "@/store/view";

/**
 * App-wide keyboard shortcuts.
 *  - Ctrl+K: command palette. Ctrl+N/O/S (+Shift+S): file actions — any view,
 *    but never while a dialog is open (Settings handles its own Ctrl+S).
 *  - Canvas editing keys only on the canvas view, with no dialog open and focus
 *    outside text fields, so Delete in Settings or Ctrl+C on selected text never
 *    touch the (hidden) graph.
 */
export function KeyboardShortcuts() {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.isComposing || e.defaultPrevented) return;
      const mod = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();
      const modal = isAnyModalOpen();
      const view = useViewStore.getState().view;

      if (mod && key === "k") {
        const palette = useCommandPaletteStore.getState();
        if (modal && !palette.open) return;
        e.preventDefault();
        palette.toggle();
        return;
      }
      if (mod && (key === "s" || key === "o" || key === "n")) {
        if (modal || (key === "s" && view === "settings")) return;
        e.preventDefault();
        if (key === "s") void (e.shiftKey ? saveFlowAs() : saveFlow());
        else if (key === "o") void openFlow();
        else void newFlow();
        return;
      }

      if (modal || view !== "canvas" || isTextInput(e.target)) return;
      const g = useGraphStore.getState();

      if (mod && key === "enter") {
        e.preventDefault();
        void runGraph();
      } else if (mod && key === "c") {
        // Selected text (an output preview, a log line…) copies as text.
        if (!hasTextSelection() && copySelection()) e.preventDefault();
      } else if (mod && key === "x") {
        if (!hasTextSelection() && cutSelection()) e.preventDefault();
      } else if (mod && key === "v") {
        if (pasteClipboard()) e.preventDefault();
      } else if (mod && key === "d") {
        if (duplicateSelection()) e.preventDefault();
      } else if (mod && key === "z") {
        e.preventDefault();
        if (e.shiftKey) g.redo();
        else g.undo();
      } else if (mod && key === "y") {
        e.preventDefault();
        g.redo();
      } else if (mod && key === "a") {
        e.preventDefault();
        g.selectAll();
      } else if (key === "delete" || key === "backspace") {
        if (g.deleteSelection()) e.preventDefault();
      } else if (key === "escape") {
        g.deselectAll();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return null;
}
