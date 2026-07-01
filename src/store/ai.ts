import { create } from "zustand";

/** Whether the "AI 生成流程" dialog is open. */
export const useAiStore = create<{ open: boolean; setOpen: (o: boolean) => void }>((set) => ({
  open: false,
  setOpen: (open) => set({ open }),
}));
