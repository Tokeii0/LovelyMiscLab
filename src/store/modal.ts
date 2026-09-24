import { useEffect, useRef } from "react";
import { create } from "zustand";

/** Stack of open modal layers (dialogs, palette, image viewer…). The topmost
 * one owns Esc, and canvas shortcuts stay off while any is open. */
interface ModalState {
  stack: number[];
  push: () => number;
  remove: (id: number) => void;
}

let seq = 0;

export const useModalStore = create<ModalState>((set, get) => ({
  stack: [],
  push: () => {
    const id = ++seq;
    set({ stack: [...get().stack, id] });
    return id;
  },
  remove: (id) => set({ stack: get().stack.filter((x) => x !== id) }),
}));

export function isAnyModalOpen(): boolean {
  return useModalStore.getState().stack.length > 0;
}

export function isTopModal(id: number): boolean {
  const s = useModalStore.getState().stack;
  return s[s.length - 1] === id;
}

/** Register a modal layer while `open`; returns a ref holding its stack id. */
export function useModalLayer(open: boolean) {
  const id = useRef<number | null>(null);
  useEffect(() => {
    if (!open) return;
    const mine = useModalStore.getState().push();
    id.current = mine;
    return () => {
      useModalStore.getState().remove(mine);
      id.current = null;
    };
  }, [open]);
  return id;
}

/** Close the topmost layer on Esc. Capture phase so it wins over global shortcuts. */
export function useEscapeToClose(open: boolean, onClose: () => void, enabled = true) {
  const layer = useModalLayer(open);
  const cb = useRef(onClose);
  cb.current = onClose;
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.isComposing) return;
      if (layer.current == null || !isTopModal(layer.current)) return;
      e.preventDefault();
      e.stopPropagation();
      if (enabled) cb.current();
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [open, enabled, layer]);
  return layer;
}
