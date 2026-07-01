import { useEffect } from "react";
import { ReactFlowProvider } from "@xyflow/react";

import { api } from "@/lib/bindings";
import { inTauri, mockDescriptors, seedDemo } from "@/lib/devMocks";
import { useDescriptorStore } from "@/store/descriptors";
import { usePaletteDrag } from "@/store/paletteDrag";
import { useViewStore } from "@/store/view";
import { LeftRail } from "@/app/LeftRail";
import { LiveRunner } from "@/app/LiveRunner";
import { TitleBar } from "@/app/TitleBar";
import { WindowResizeHandles } from "@/app/WindowResizeHandles";
import { CanvasView } from "@/views/CanvasView";
import { ResourcesView, RunsView } from "@/views/EmptyState";
import { ModulesView } from "@/views/ModulesView";
import { SettingsView } from "@/views/SettingsView";
import { TemplatesView } from "@/views/TemplatesView";

function DragGhost() {
  const descriptor = usePaletteDrag((s) => s.descriptor);
  const x = usePaletteDrag((s) => s.x);
  const y = usePaletteDrag((s) => s.y);
  if (!descriptor) return null;
  return (
    <div
      className="pointer-events-none fixed z-[60] -translate-x-1/2 -translate-y-1/2 rounded-md border border-primary bg-card px-2 py-1 text-xs font-medium shadow-lg"
      style={{ left: x, top: y }}
    >
      {descriptor.displayName}
    </div>
  );
}

function App() {
  const view = useViewStore((s) => s.view);
  const setDescriptors = useDescriptorStore((s) => s.setDescriptors);

  useEffect(() => {
    if (inTauri) {
      api
        .listNodeDescriptors()
        .then(setDescriptors)
        .catch((e) => console.error("listNodeDescriptors failed", e));
      return;
    }
    // Browser dev preview (no Tauri IPC): seed mocks + a demo graph.
    setDescriptors(mockDescriptors);
    seedDemo();
  }, [setDescriptors]);

  // Suppress the native webview context menu (except in text fields) so our
  // canvas right-click menu is the only one shown.
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) {
        return;
      }
      e.preventDefault();
    };
    document.addEventListener("contextmenu", handler);
    return () => document.removeEventListener("contextmenu", handler);
  }, []);

  // Drive the custom palette→canvas drag from global pointer events.
  useEffect(() => {
    const move = (e: PointerEvent) => {
      const s = usePaletteDrag.getState();
      if (s.descriptor) s.move(e.clientX, e.clientY);
    };
    const up = (e: PointerEvent) => {
      const s = usePaletteDrag.getState();
      if (s.descriptor && s.drop) {
        const moved = Math.hypot(e.clientX - s.startX, e.clientY - s.startY) > 6;
        s.drop(s.descriptor, e.clientX, e.clientY, moved);
      }
      if (s.descriptor) s.clear();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, []);

  return (
    <ReactFlowProvider>
      <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
        <TitleBar />
        <div className="flex min-h-0 flex-1">
          <LeftRail />
          <div className="min-w-0 flex-1">
            {view === "canvas" && <CanvasView />}
            {view === "modules" && <ModulesView />}
            {view === "templates" && <TemplatesView />}
            {view === "runs" && <RunsView />}
            {view === "resources" && <ResourcesView />}
            {view === "settings" && <SettingsView />}
          </div>
        </div>
        <WindowResizeHandles />
        <DragGhost />
        <LiveRunner />
      </div>
    </ReactFlowProvider>
  );
}

export default App;
