import { create } from "zustand";

/** Whether the "封装为模块" (create composite module) dialog is open. */
export const useModuleDialogStore = create<{ open: boolean; setOpen: (o: boolean) => void }>(
  (set) => ({
    open: false,
    setOpen: (open) => set({ open }),
  })
);
