import { create } from "zustand";

import { api, type UpdateInfo } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";

type Status = "idle" | "checking" | "available" | "uptodate" | "installing" | "error";

function errText(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return String(e);
}

interface UpdateState {
  status: Status;
  info: UpdateInfo | null;
  error: string;
  dialogOpen: boolean;
  /** Query GitHub for the latest release. `silent` = don't surface "up to date". */
  check: (opts?: { silent?: boolean }) => Promise<void>;
  /** Download + swap the new exe; the app relaunches on success. */
  install: () => Promise<void>;
  closeDialog: () => void;
}

export const useUpdate = create<UpdateState>((set, get) => ({
  status: "idle",
  info: null,
  error: "",
  dialogOpen: false,

  check: async ({ silent } = {}) => {
    if (!inTauri) {
      set({ status: "error", error: "自动更新仅在桌面应用内可用。" });
      return;
    }
    set({ status: "checking", error: "" });
    try {
      const info = await api.checkUpdate();
      if (info.available) {
        set({ status: "available", info, dialogOpen: true });
      } else {
        set({ status: "uptodate", info, dialogOpen: silent ? get().dialogOpen : true });
      }
    } catch (e) {
      set({ status: "error", error: errText(e), dialogOpen: silent ? get().dialogOpen : true });
    }
  },

  install: async () => {
    const info = get().info;
    if (!info || !info.downloadUrl) {
      set({ status: "error", error: "该版本没有可下载的可执行文件。" });
      return;
    }
    set({ status: "installing", error: "" });
    try {
      // On success the backend swaps the exe and relaunches, so this call
      // typically never resolves — the whole app restarts.
      await api.installUpdate(info.downloadUrl);
    } catch (e) {
      set({ status: "error", error: errText(e) });
    }
  },

  closeDialog: () => set({ dialogOpen: false }),
}));
