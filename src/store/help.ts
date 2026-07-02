import { create } from "zustand";

interface HelpState {
  open: boolean;
  descriptorId: string | null;
  setOpen: (open: boolean) => void;
  openForNode: (descriptorId?: string | null) => void;
}

export const useHelpStore = create<HelpState>((set) => ({
  open: false,
  descriptorId: null,
  setOpen: (open) => set({ open }),
  openForNode: (descriptorId) => set({ open: true, descriptorId: descriptorId ?? null }),
}));
