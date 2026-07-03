import { useEffect, useRef, useState } from "react";

import { RunConsole } from "@/app/RunConsole";
import { Canvas } from "@/flow/Canvas";
import { Inspector } from "@/flow/Inspector";
import { ModuleLibrary } from "@/flow/ModuleLibrary";

const WIDTH_KEY = "misclab-palette-width";
const MIN_W = 180;
const MAX_W = 520;

function usePaletteWidth() {
  const [w, setW] = useState(() => {
    const v = parseInt(localStorage.getItem(WIDTH_KEY) || "", 10);
    return Number.isFinite(v) && v >= MIN_W && v <= MAX_W ? v : 240;
  });
  useEffect(() => {
    try {
      localStorage.setItem(WIDTH_KEY, String(w));
    } catch {
      /* ignore */
    }
  }, [w]);
  return [w, setW] as const;
}

export function CanvasView() {
  const [width, setWidth] = usePaletteWidth();
  const asideRef = useRef<HTMLElement>(null);

  const startResize = (e: React.PointerEvent) => {
    e.preventDefault();
    const onMove = (ev: PointerEvent) => {
      const left = asideRef.current?.getBoundingClientRect().left ?? 0;
      setWidth(Math.min(MAX_W, Math.max(MIN_W, Math.round(ev.clientX - left))));
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1">
        <aside
          ref={asideRef}
          style={{ width }}
          className="relative shrink-0 border-r border-border"
        >
          <ModuleLibrary />
          <div
            onPointerDown={startResize}
            onDoubleClick={() => setWidth(240)}
            title="拖动调整宽度（双击重置）"
            className="absolute right-0 top-0 z-10 h-full w-1.5 cursor-col-resize hover:bg-primary/40 active:bg-primary/60"
          />
        </aside>
        <main className="min-w-0 flex-1">
          <Canvas />
        </main>
        <aside className="w-72 shrink-0 border-l border-border bg-card">
          <Inspector />
        </aside>
      </div>
      <RunConsole />
    </div>
  );
}
