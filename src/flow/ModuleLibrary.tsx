import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, RotateCw } from "lucide-react";

import type { NodeDescriptor, PortSpec } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { usePaletteDrag } from "@/store/paletteDrag";

import { nodeIcon } from "./nodeIcons";
import { portColor } from "./portColors";

const COLLAPSE_KEY = "misclab-collapsed-categories";

function Dots({ ports }: { ports: PortSpec[] }) {
  if (ports.length === 0) return <span className="text-muted-foreground/40">无</span>;
  return (
    <span className="flex gap-0.5">
      {ports.map((p) => (
        <span
          key={p.name}
          title={`${p.label}: ${p.type}`}
          className="h-1.5 w-1.5 rounded-full"
          style={{ background: portColor(p.type) }}
        />
      ))}
    </span>
  );
}

export function ModuleLibrary() {
  const list = useDescriptorStore((s) => s.list);
  const [q, setQ] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(() => {
    try {
      return new Set(JSON.parse(localStorage.getItem(COLLAPSE_KEY) || "[]"));
    } catch {
      return new Set();
    }
  });

  const toggle = (c: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      next.has(c) ? next.delete(c) : next.add(c);
      try {
        localStorage.setItem(COLLAPSE_KEY, JSON.stringify([...next]));
      } catch {
        /* ignore */
      }
      return next;
    });

  const grouped = useMemo(() => {
    const needle = q.toLowerCase();
    const filtered = list.filter(
      (d) =>
        d.displayName.toLowerCase().includes(needle) ||
        d.category.toLowerCase().includes(needle)
    );
    const map = new Map<string, NodeDescriptor[]>();
    for (const d of filtered) {
      const arr = map.get(d.category) ?? [];
      arr.push(d);
      map.set(d.category, arr);
    }
    return Array.from(map.entries());
  }, [list, q]);

  const onPointerDown = (e: React.PointerEvent, d: NodeDescriptor) => {
    if (e.button !== 0) return;
    e.preventDefault();
    usePaletteDrag.getState().start(d, e.clientX, e.clientY);
  };

  const searching = q.trim().length > 0;

  return (
    <div className="flex h-full flex-col bg-card">
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <span className="text-xs font-semibold">模块库</span>
        <RotateCw className="h-3.5 w-3.5 text-muted-foreground" />
      </div>
      <div className="border-b border-border p-2">
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="搜索模块…"
          className="w-full rounded-md border border-input bg-background px-2 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
        />
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        {grouped.map(([category, nodes]) => {
          const isCollapsed = !searching && collapsed.has(category);
          return (
            <div key={category} className="mb-2">
              <button
                onClick={() => toggle(category)}
                className="flex w-full items-center gap-1 rounded px-1 py-1 text-[11px] font-semibold text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              >
                {isCollapsed ? (
                  <ChevronRight className="h-3 w-3 shrink-0" />
                ) : (
                  <ChevronDown className="h-3 w-3 shrink-0" />
                )}
                <span className="flex-1 truncate text-left">{category}</span>
                <span className="text-[10px] opacity-60">{nodes.length}</span>
              </button>
              {!isCollapsed && (
                <div className="grid grid-cols-2 gap-1.5 p-1">
                  {nodes.map((d) => {
                    const Icon = nodeIcon(d.id, d.category);
                    return (
                      <div
                        key={d.id}
                        onPointerDown={(e) => onPointerDown(e, d)}
                        title={d.description || d.displayName}
                        className="cursor-grab touch-none select-none rounded-lg border border-border bg-background p-2 transition-all hover:border-primary hover:shadow-sm active:cursor-grabbing"
                      >
                        <div className="flex items-center gap-1.5">
                          <span
                            className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md"
                            style={{ background: `${d.color}18`, color: d.color }}
                          >
                            <Icon className="h-3.5 w-3.5" />
                          </span>
                          <span className="truncate text-[11px] font-medium">
                            {d.displayName}
                          </span>
                        </div>
                        <div className="mt-1.5 flex items-center gap-1 text-[9px] text-muted-foreground">
                          <Dots ports={d.inputs} />
                          <span className="opacity-50">→</span>
                          <Dots ports={d.outputs} />
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
        {grouped.length === 0 && (
          <div className="p-2 text-xs text-muted-foreground">无匹配模块</div>
        )}
      </div>
      <div className="border-t border-border px-3 py-1.5 text-[10px] text-muted-foreground">
        拖拽或点击添加到画布
      </div>
    </div>
  );
}
