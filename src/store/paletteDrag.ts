import { create } from "zustand";

import type { NodeDescriptor } from "@/lib/types";

type Drop = (d: NodeDescriptor, x: number, y: number, moved: boolean) => void;

// Custom palette→canvas drag state. HTML5 drag-and-drop is unreliable inside the
// Tauri (WebView2) window, so we drive it with pointer events instead.
interface PaletteDragState {
  descriptor: NodeDescriptor | null;
  x: number;
  y: number;
  startX: number;
  startY: number;
  drop: Drop | null;
  start: (d: NodeDescriptor, x: number, y: number) => void;
  move: (x: number, y: number) => void;
  clear: () => void;
  setDrop: (fn: Drop) => void;
}

export const usePaletteDrag = create<PaletteDragState>((set) => ({
  descriptor: null,
  x: 0,
  y: 0,
  startX: 0,
  startY: 0,
  drop: null,
  start: (d, x, y) => set({ descriptor: d, x, y, startX: x, startY: y }),
  move: (x, y) => set({ x, y }),
  clear: () => set({ descriptor: null }),
  setDrop: (fn) => set({ drop: fn }),
}));
