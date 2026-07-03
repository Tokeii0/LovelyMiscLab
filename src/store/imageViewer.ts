import { create } from "zustand";

/**
 * Global lightbox for any image shown in the app (node preview, inspector,
 * run history, input widget). Opened imperatively via `getState().show(src)`
 * so the many callers don't subscribe to the store and re-render needlessly.
 */
interface ImageViewerState {
  open: boolean;
  src: string | null;
  title: string;
  show: (src: string, title?: string) => void;
  close: () => void;
}

export const useImageViewer = create<ImageViewerState>((set) => ({
  open: false,
  src: null,
  title: "",
  show: (src, title = "图片查看") => set({ open: true, src, title }),
  close: () => set({ open: false, src: null }),
}));
