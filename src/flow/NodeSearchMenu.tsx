import { useEffect, useMemo, useRef, useState } from "react";

import type { NodeDescriptor } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";

import { portColor } from "./portColors";

interface Props {
  x: number;
  y: number;
  onPick: (d: NodeDescriptor) => void;
  onClose: () => void;
}

/** ComfyUI-style searchable node picker, opened at the cursor. */
export function NodeSearchMenu({ x, y, onPick, onClose }: Props) {
  const list = useDescriptorStore((s) => s.list);
  const [q, setQ] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const results = useMemo(() => {
    const needle = q.toLowerCase();
    return list.filter(
      (d) =>
        d.displayName.toLowerCase().includes(needle) ||
        d.category.toLowerCase().includes(needle) ||
        d.id.toLowerCase().includes(needle)
    );
  }, [list, q]);

  const left = Math.min(x, window.innerWidth - 244);
  const top = Math.min(y, window.innerHeight - 340);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") onClose();
    else if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((a) => Math.min(a + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((a) => Math.max(a - 1, 0));
    } else if (e.key === "Enter" && results[active]) {
      onPick(results[active]);
    }
  };

  return (
    <>
      <div
        className="fixed inset-0 z-40"
        onClick={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
      />
      <div
        className="fixed z-50 w-60 overflow-hidden rounded-md border border-border bg-popover text-popover-foreground shadow-xl"
        style={{ left, top }}
      >
        <div className="border-b border-border p-1.5">
          <input
            ref={inputRef}
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setActive(0);
            }}
            onKeyDown={onKeyDown}
            placeholder="搜索节点…"
            className="w-full rounded border border-input bg-background px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
        <div className="max-h-72 overflow-y-auto p-1">
          {results.map((d, i) => (
            <button
              key={d.id}
              onMouseEnter={() => setActive(i)}
              onClick={() => onPick(d)}
              className={`flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-xs ${
                i === active ? "bg-accent" : "hover:bg-accent"
              }`}
            >
              <span
                className="h-2.5 w-2.5 shrink-0 rounded-full"
                style={{ background: portColor(d.outputs[0]?.type ?? "any") }}
              />
              <span className="flex-1 truncate">{d.displayName}</span>
              <span className="shrink-0 text-[10px] text-muted-foreground">
                {d.category}
              </span>
            </button>
          ))}
          {results.length === 0 && (
            <div className="px-2 py-3 text-center text-xs text-muted-foreground">
              无匹配节点
            </div>
          )}
        </div>
      </div>
    </>
  );
}
