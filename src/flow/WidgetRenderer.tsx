import { type ChangeEvent, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import type { ParamSpec } from "@/lib/types";
import { inTauri } from "@/lib/devMocks";
import { useImageViewer } from "@/store/imageViewer";
import { toast } from "@/store/toast";

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

function ImageField({
  value,
  onChange,
}: {
  value: unknown;
  onChange: (v: unknown) => void;
}) {
  const url = typeof value === "string" ? value : "";
  const ref = useRef<HTMLInputElement>(null);
  const onFile = (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => onChange(reader.result as string);
    reader.onerror = () => toast.error("读取图片失败", { error: reader.error });
    reader.readAsDataURL(file);
    e.target.value = ""; // allow re-picking the same file
  };
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-1">
        <button
          onClick={() => ref.current?.click()}
          className="nodrag shrink-0 rounded border border-input bg-background px-1.5 py-0.5 text-[10px] hover:bg-accent"
        >
          选择图片
        </button>
        {url && (
          <button
            onClick={() => onChange("")}
            className="nodrag shrink-0 rounded border border-input px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-accent"
          >
            清除
          </button>
        )}
        <input ref={ref} type="file" accept="image/*" className="hidden" onChange={onFile} />
      </div>
      {url && (
        <img
          src={url}
          alt=""
          title="点击查看大图 / 调整"
          onClick={() => useImageViewer.getState().show(url)}
          className="nodrag max-h-40 w-full cursor-zoom-in rounded border border-border bg-white object-contain"
        />
      )}
    </div>
  );
}

/** Number input that keeps what's being typed ("", "-", "1.") as a draft and only
 * commits real numbers — so negative/decimal entry works and NaN never reaches
 * the graph. Out-of-range values are clamped when the field loses focus. */
function NumberField({
  value,
  min,
  max,
  step,
  onChange,
}: {
  value: unknown;
  min: number;
  max: number;
  step: number;
  onChange: (v: unknown) => void;
}) {
  const current = Number(value ?? 0);
  const [draft, setDraft] = useState(String(current));
  const focused = useRef(false);
  useEffect(() => {
    if (!focused.current) setDraft(String(current));
  }, [current]);
  const parse = (t: string) => (t.trim() === "" ? NaN : Number(t));
  return (
    <input
      type="number"
      className={base}
      min={min}
      max={max}
      step={step}
      value={draft}
      onFocus={() => (focused.current = true)}
      onChange={(e) => {
        setDraft(e.target.value);
        const n = parse(e.target.value);
        if (Number.isFinite(n)) onChange(n);
      }}
      onBlur={() => {
        focused.current = false;
        const n = parse(draft);
        const next = Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : current;
        setDraft(String(next));
        if (next !== current) onChange(next);
      }}
    />
  );
}

export function WidgetRenderer({ spec, value, onChange }: Props) {
  const w = spec.widget;
  switch (w.kind) {
    case "text":
      return w.multiline ? (
        <textarea
          className={`${base} nopan resize-none`}
          // Grows with the content (3–12 rows) so pasted ciphertext stays readable.
          rows={Math.min(12, Math.max(3, String(value ?? "").split("\n").length))}
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
      return <NumberField value={value} min={w.min} max={w.max} step={w.step} onChange={onChange} />;
    case "slider":
      return (
        <div className="flex items-center gap-2">
          <input
            type="range"
            className="nodrag min-w-0 flex-1"
            min={w.min}
            max={w.max}
            step={w.step}
            value={Number(value ?? 0)}
            onChange={(e) => onChange(parseFloat(e.target.value))}
          />
          <span className="w-8 shrink-0 text-right font-mono text-[10px] text-muted-foreground">
            {Number(value ?? 0)}
          </span>
        </div>
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
    case "image":
      return <ImageField value={value} onChange={onChange} />;
  }
}
