import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { inTauri } from "@/lib/devMocks";

/** Whether the (frameless) main window is currently maximized. */
export function useWindowMaximized(): boolean {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    if (!inTauri) return;
    const w = getCurrentWindow();
    const sync = () => void w.isMaximized().then(setMaximized).catch(() => {});
    sync();
    const unlisten = w.onResized(sync);
    return () => {
      unlisten.then((f) => f()).catch(() => {});
    };
  }, []);
  return maximized;
}
