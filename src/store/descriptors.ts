import { create } from "zustand";

import type { NodeDescriptor } from "@/lib/types";

interface DescriptorState {
  list: NodeDescriptor[];
  byId: Record<string, NodeDescriptor>;
  setDescriptors: (list: NodeDescriptor[]) => void;
}

/** Node descriptors loaded once at startup; drives palette + generic rendering. */
export const useDescriptorStore = create<DescriptorState>((set) => ({
  list: [],
  byId: {},
  setDescriptors: (list) =>
    set({
      list,
      byId: Object.fromEntries(list.map((d) => [d.id, d])),
    }),
}));
