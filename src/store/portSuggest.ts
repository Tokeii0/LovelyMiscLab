import { create } from "zustand";

/** Which port opened the suggestion panel, and where to anchor it (screen px). */
export interface PortSuggestCtx {
  nodeId: string;
  descriptorId: string;
  port: string;
  dir: "in" | "out";
  anchor: { x: number; y: number };
}

interface PortSuggestState {
  ctx: PortSuggestCtx | null;
  open: (ctx: PortSuggestCtx) => void;
  close: () => void;
}

/** Drives the port-hover ✨ next-node suggestion panel (opened from GenericNode,
 * rendered by Canvas so it has ReactFlow + graph-store access). */
export const usePortSuggest = create<PortSuggestState>((set) => ({
  ctx: null,
  open: (ctx) => set({ ctx }),
  close: () => set({ ctx: null }),
}));
