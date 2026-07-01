import { open } from "@tauri-apps/plugin-dialog";

import type { ParamSpec } from "@/lib/types";
import { inTauri } from "@/lib/devMocks";

interface Props {
  spec: ParamSpec;
  value: unknown;
  onChange: (value: unknown) => void;
}

// `nodrag`/`nowheel`/`nopan` keep interactions from panning/zooming the canvas.
const base =
  "nodrag nowheel w-full rounded border border-input bg-background px-1.5 py-0.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-ring";

function FileField({
  value,
  onChange,
}: {
  value: unknown;
  onChange: (v: unknown) => void;
}) {
  const path = typeof value === "string" ? value : "";
  const name = path ? path.split(/[\\/]/).pop() : "未选择";
  const pick = async () => {
    if (!inTauri) return;
    const selected = await open({ multiple: false, directory: false });
    if (typeof selected === "string") onChange(selected);
  };
  return (
    <div className="flex items-center gap-1">
      <button
        onClick={pick}
        className="nodrag shrink-0 rounded border border-input bg-background px-1.5 py-0.5 text-[10px] hover:bg-accent"
      >
        选择文件
      </button>
      <span className="truncate text-[10px] text-muted-foreground" title={path}>
        {name}
      </span>
    </div>
  );
}

export function WidgetRenderer({ spec, value, onChange }: Props) {
  const w = spec.widget;
  switch (w.kind) {
    case "text":
      return w.multiline ? (
        <textarea
          className={`${base} nopan resize-none`}
          rows={3}
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : (
        <input
          className={base}
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
        />
      );
    case "number":
      return (
        <input
          type="number"
          className={base}
          min={w.min}
          max={w.max}
          step={w.step}
          value={Number(value ?? 0)}
          onChange={(e) => onChange(parseFloat(e.target.value))}
        />
      );
    case "slider":
      return (
        <input
          type="range"
          className="nodrag w-full"
          min={w.min}
          max={w.max}
          step={w.step}
          value={Number(value ?? 0)}
          onChange={(e) => onChange(parseFloat(e.target.value))}
        />
      );
    case "select":
      return (
        <select
          className={base}
          value={String(value ?? "")}
          onChange={(e) => onChange(e.target.value)}
        >
          {w.options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      );
    case "toggle":
      return (
        <input
          type="checkbox"
          className="nodrag h-4 w-4"
          checked={Boolean(value)}
          onChange={(e) => onChange(e.target.checked)}
        />
      );
    case "file":
      return <FileField value={value} onChange={onChange} />;
  }
}
