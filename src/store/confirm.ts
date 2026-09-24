import { create } from "zustand";

/** Promise-based replacements for window.confirm / window.prompt, rendered by
 * <ConfirmHost/> in the app's own style. Requests queue up one at a time. */

export interface ChoiceOption<T> {
  value: T;
  label: string;
  variant?: "default" | "outline" | "destructive";
}

type Request = { id?: number } & (
  | {
      kind: "confirm";
      title: string;
      message?: string;
      confirmText: string;
      danger: boolean;
      resolve: (v: boolean) => void;
    }
  | {
      kind: "prompt";
      title: string;
      message?: string;
      initial: string;
      placeholder?: string;
      confirmText: string;
      resolve: (v: string | null) => void;
    }
  | {
      kind: "choose";
      title: string;
      message?: string;
      options: ChoiceOption<unknown>[];
      resolve: (v: unknown) => void;
    }
);

let seq = 0;

interface ConfirmState {
  queue: Request[];
  enqueue: (r: Request) => void;
  /** Resolve the current request with `value` (null / false = cancelled). */
  settle: (value: unknown) => void;
}

export const useConfirmStore = create<ConfirmState>((set, get) => ({
  queue: [],
  enqueue: (r) => set({ queue: [...get().queue, { ...r, id: ++seq }] }),
  settle: (value) => {
    const [head, ...rest] = get().queue;
    if (!head) return;
    set({ queue: rest });
    if (head.kind === "confirm") head.resolve(value === true);
    else if (head.kind === "prompt") head.resolve(typeof value === "string" ? value : null);
    else head.resolve(value ?? null);
  },
}));

export function confirmDialog(opts: {
  title: string;
  message?: string;
  confirmText?: string;
  danger?: boolean;
}): Promise<boolean> {
  return new Promise((resolve) =>
    useConfirmStore.getState().enqueue({
      kind: "confirm",
      title: opts.title,
      message: opts.message,
      confirmText: opts.confirmText ?? "确定",
      danger: opts.danger ?? false,
      resolve,
    })
  );
}

export function promptDialog(opts: {
  title: string;
  message?: string;
  initial?: string;
  placeholder?: string;
  confirmText?: string;
}): Promise<string | null> {
  return new Promise((resolve) =>
    useConfirmStore.getState().enqueue({
      kind: "prompt",
      title: opts.title,
      message: opts.message,
      initial: opts.initial ?? "",
      placeholder: opts.placeholder,
      confirmText: opts.confirmText ?? "确定",
      resolve,
    })
  );
}

/** Ask the user to pick one of several actions; resolves null on cancel/Esc. */
export function choose<T>(opts: {
  title: string;
  message?: string;
  options: ChoiceOption<T>[];
}): Promise<T | null> {
  return new Promise((resolve) =>
    useConfirmStore.getState().enqueue({
      kind: "choose",
      title: opts.title,
      message: opts.message,
      options: opts.options as ChoiceOption<unknown>[],
      resolve: resolve as (v: unknown) => void,
    })
  );
}
