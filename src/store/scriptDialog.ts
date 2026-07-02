import { create } from "zustand";

/** Whether the "脚本节点" (create external-script node) dialog is open. */
export const useScriptDialogStore = create<{ open: boolean; setOpen: (o: boolean) => void }>(
  (set) => ({
    open: false,
    setOpen: (open) => set({ open }),
  })
);
