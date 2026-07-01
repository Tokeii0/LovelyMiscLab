import { RunConsole } from "@/app/RunConsole";
import { Canvas } from "@/flow/Canvas";
import { Inspector } from "@/flow/Inspector";
import { ModuleLibrary } from "@/flow/ModuleLibrary";

export function CanvasView() {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex min-h-0 flex-1">
        <aside className="w-60 shrink-0 border-r border-border">
          <ModuleLibrary />
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
