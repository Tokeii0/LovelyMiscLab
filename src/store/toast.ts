import { create } from "zustand";

import { errorMessage } from "@/lib/errors";

export type ToastKind = "success" | "info" | "error";

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface ToastItem {
  id: number;
  kind: ToastKind;
  message: string;
  detail?: string;
  actions: ToastAction[];
  /** Auto-dismiss after this many ms; 0 = stays until dismissed. */
  duration: number;
}

export interface ToastOptions {
  /** Secondary line under the message. */
  detail?: string;
  /** Rendered through `errorMessage` into `detail` when no `detail` is given. */
  error?: unknown;
  actions?: ToastAction[];
  duration?: number;
}

interface ToastState {
  items: ToastItem[];
  push: (kind: ToastKind, message: string, opts?: ToastOptions) => number;
  dismiss: (id: number) => void;
}

const MAX_VISIBLE = 4;
const DEFAULT_MS: Record<ToastKind, number> = { success: 3000, info: 4000, error: 8000 };
let seq = 0;

export const useToastStore = create<ToastState>((set, get) => ({
  items: [],
  push: (kind, message, opts = {}) => {
    const id = ++seq;
    const actions = opts.actions ?? [];
    const detail = opts.detail ?? (opts.error !== undefined ? errorMessage(opts.error) : undefined);
    // Toasts with buttons stay a little longer so there is time to act on them.
    const duration = opts.duration ?? (actions.length ? 10000 : DEFAULT_MS[kind]);
    set({ items: [...get().items, { id, kind, message, detail, actions, duration }].slice(-MAX_VISIBLE) });
    if (duration > 0) setTimeout(() => get().dismiss(id), duration);
    return id;
  },
  dismiss: (id) => set({ items: get().items.filter((t) => t.id !== id) }),
}));

/** Fire-and-forget notifications, callable from components and plain modules alike. */
export const toast = {
  success: (message: string, opts?: ToastOptions) => useToastStore.getState().push("success", message, opts),
  info: (message: string, opts?: ToastOptions) => useToastStore.getState().push("info", message, opts),
  error: (message: string, opts?: ToastOptions) => useToastStore.getState().push("error", message, opts),
  dismiss: (id: number) => useToastStore.getState().dismiss(id),
};
