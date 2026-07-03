import { useCallback, useEffect, useRef, useState } from "react";
import {
  Aperture,
  Contrast,
  Crosshair,
  Droplet,
  FlipHorizontal2,
  FlipVertical2,
  Maximize2,
  RefreshCw,
  RotateCcw,
  RotateCw,
  SlidersHorizontal,
  Sparkles,
  Sun,
  X,
  ZoomIn,
  ZoomOut,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { useImageViewer } from "@/store/imageViewer";

const MIN_SCALE = 0.05;
const MAX_SCALE = 40;

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));

interface Adjust {
  brightness: number;
  contrast: number;
  saturate: number;
  exposure: number; // EV-like gamma stops; 0 = identity
  sharpen: number; // unsharp strength; 0 = off
}

const DEFAULT_ADJUST: Adjust = {
  brightness: 1,
  contrast: 1,
  saturate: 1,
  exposure: 0,
  sharpen: 0,
};

/**
 * Full-screen image lightbox. Pan/zoom (drag + wheel), 90°/180° rotation,
 * horizontal/vertical flip, and live Photoshop-style adjustments (brightness,
 * contrast, exposure, saturation, sharpen) applied purely with CSS + SVG
 * filters so nothing is re-encoded. Mounted once at the app root.
 */
export function ImageViewerModal() {
  const open = useImageViewer((s) => s.open);
  const src = useImageViewer((s) => s.src);
  const title = useImageViewer((s) => s.title);
  const close = useImageViewer((s) => s.close);

  const stageRef = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; ox: number; oy: number; moved: boolean } | null>(
    null
  );

  const [scale, setScale] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [rot, setRot] = useState(0);
  const [flipH, setFlipH] = useState(false);
  const [flipV, setFlipV] = useState(false);
  const [adj, setAdj] = useState<Adjust>(DEFAULT_ADJUST);
  const [nat, setNat] = useState({ w: 0, h: 0 });

  const fitToStage = useCallback(() => {
    const el = stageRef.current;
    if (!el || !nat.w || !nat.h) {
      setScale(1);
      setOffset({ x: 0, y: 0 });
      return;
    }
    const pad = 56;
    const s = Math.min((el.clientWidth - pad) / nat.w, (el.clientHeight - pad) / nat.h, 1);
    setScale(s > 0 ? s : 1);
    setOffset({ x: 0, y: 0 });
  }, [nat]);

  const resetOrientation = useCallback(() => {
    setRot(0);
    setFlipH(false);
    setFlipV(false);
    setOffset({ x: 0, y: 0 });
  }, []);

  const resetAll = useCallback(() => {
    resetOrientation();
    setAdj(DEFAULT_ADJUST);
    fitToStage();
  }, [resetOrientation, fitToStage]);

  const zoomBy = useCallback((factor: number) => {
    setScale((s) => clamp(s * factor, MIN_SCALE, MAX_SCALE));
  }, []);

  // Fresh state whenever a new image is opened.
  useEffect(() => {
    if (!open) return;
    setScale(1);
    setOffset({ x: 0, y: 0 });
    setRot(0);
    setFlipH(false);
    setFlipV(false);
    setAdj(DEFAULT_ADJUST);
    setNat({ w: 0, h: 0 });
  }, [open, src]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
      else if (e.key === "+" || e.key === "=") zoomBy(1.2);
      else if (e.key === "-" || e.key === "_") zoomBy(1 / 1.2);
      else if (e.key === "0") fitToStage();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, close, zoomBy, fitToStage]);

  if (!open || !src) return null;

  const onImgLoad = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    const w = img.naturalWidth;
    const h = img.naturalHeight;
    setNat({ w, h });
    const el = stageRef.current;
    const pad = 56;
    const s = el ? Math.min((el.clientWidth - pad) / w, (el.clientHeight - pad) / h, 1) : 1;
    setScale(s > 0 ? s : 1);
    setOffset({ x: 0, y: 0 });
  };

  // Cursor-anchored wheel zoom (outer transform has no rotation, so the maths
  // stay exact regardless of flip/rotate applied on the inner <img>).
  const onWheel = (e: React.WheelEvent) => {
    const el = stageRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const cx = e.clientX - (rect.left + rect.width / 2);
    const cy = e.clientY - (rect.top + rect.height / 2);
    const f = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    const next = clamp(scale * f, MIN_SCALE, MAX_SCALE);
    const r = next / scale;
    setScale(next);
    setOffset({ x: cx * (1 - r) + r * offset.x, y: cy * (1 - r) + r * offset.y });
  };

  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    drag.current = { x: e.clientX, y: e.clientY, ox: offset.x, oy: offset.y, moved: false };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: React.PointerEvent) => {
    const d = drag.current;
    if (!d) return;
    const dx = e.clientX - d.x;
    const dy = e.clientY - d.y;
    if (Math.hypot(dx, dy) > 3) d.moved = true;
    setOffset({ x: d.ox + dx, y: d.oy + dy });
  };
  const onPointerUp = (e: React.PointerEvent) => {
    const d = drag.current;
    drag.current = null;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      /* pointer already released */
    }
    // A click (no drag) on the empty backdrop closes the viewer.
    if (d && !d.moved && e.target === stageRef.current) close();
  };

  const expExponent = Math.pow(2, -adj.exposure);
  const k = adj.sharpen;
  const kernel = `0 ${-k} 0 ${-k} ${1 + 4 * k} ${-k} 0 ${-k} 0`;

  // Only attach the SVG filters when they actually do something — an always-on
  // feConvolveMatrix would run a per-pixel convolution every paint for nothing.
  const filter = [
    `brightness(${adj.brightness})`,
    `contrast(${adj.contrast})`,
    `saturate(${adj.saturate})`,
    adj.exposure !== 0 ? "url(#lml-iv-exposure)" : "",
    adj.sharpen > 0 ? "url(#lml-iv-sharpen)" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const outerTransform = `translate(${offset.x}px, ${offset.y}px) scale(${scale})`;
  const innerTransform = `rotate(${rot}deg) scale(${flipH ? -1 : 1}, ${flipV ? -1 : 1})`;

  const tbtn =
    "flex h-8 min-w-[2rem] items-center justify-center gap-1 rounded px-2 text-[13px] text-white/80 transition-colors hover:bg-white/10 hover:text-white";
  const tactive = "bg-white/15 text-white";

  return (
    <div className="fixed inset-0 z-[90] flex flex-col bg-black/95">
      {/* Off-screen SVG filter defs driven by the adjustment sliders. */}
      <svg aria-hidden width="0" height="0" className="absolute">
        <defs>
          <filter id="lml-iv-exposure" colorInterpolationFilters="sRGB">
            <feComponentTransfer>
              <feFuncR type="gamma" amplitude={1} exponent={expExponent} offset={0} />
              <feFuncG type="gamma" amplitude={1} exponent={expExponent} offset={0} />
              <feFuncB type="gamma" amplitude={1} exponent={expExponent} offset={0} />
            </feComponentTransfer>
          </filter>
          <filter id="lml-iv-sharpen" colorInterpolationFilters="sRGB">
            <feConvolveMatrix order={3} preserveAlpha divisor={1} kernelMatrix={kernel} />
          </filter>
        </defs>
      </svg>

      {/* Toolbar */}
      <div className="flex items-center gap-1 border-b border-white/10 px-3 py-2 text-white">
        <span className="mr-2 max-w-[24ch] truncate text-sm font-medium" title={title}>
          {title}
        </span>

        <div className="flex items-center gap-0.5">
          <button className={tbtn} title="缩小 ( - )" onClick={() => zoomBy(1 / 1.2)}>
            <ZoomOut className="h-4 w-4" />
          </button>
          <button
            className={cn(tbtn, "min-w-[3.5rem] font-mono text-xs")}
            title="重置为 100%"
            onClick={() => setScale(1)}
          >
            {Math.round(scale * 100)}%
          </button>
          <button className={tbtn} title="放大 ( + )" onClick={() => zoomBy(1.2)}>
            <ZoomIn className="h-4 w-4" />
          </button>
          <button className={tbtn} title="适应窗口 ( 0 )" onClick={fitToStage}>
            <Maximize2 className="h-4 w-4" />
          </button>
        </div>

        <div className="mx-2 h-5 w-px bg-white/15" />

        <div className="flex items-center gap-0.5">
          <button className={tbtn} title="左转 90°" onClick={() => setRot((r) => r - 90)}>
            <RotateCcw className="h-4 w-4" />
          </button>
          <button className={tbtn} title="右转 90°" onClick={() => setRot((r) => r + 90)}>
            <RotateCw className="h-4 w-4" />
          </button>
          <button
            className={cn(tbtn, flipH && tactive)}
            title="水平翻转"
            onClick={() => setFlipH((v) => !v)}
          >
            <FlipHorizontal2 className="h-4 w-4" />
          </button>
          <button
            className={cn(tbtn, flipV && tactive)}
            title="垂直翻转"
            onClick={() => setFlipV((v) => !v)}
          >
            <FlipVertical2 className="h-4 w-4" />
          </button>
          <button
            className={tbtn}
            title="中心对称 (旋转 180°)"
            onClick={() => setRot((r) => r + 180)}
          >
            <Crosshair className="h-4 w-4" />
          </button>
        </div>

        <div className="mx-2 h-5 w-px bg-white/15" />

        <button className={tbtn} title="重置全部" onClick={resetAll}>
          <RefreshCw className="h-4 w-4" />
          <span className="text-xs">重置</span>
        </button>

        <div className="flex-1" />
        <button className={tbtn} title="关闭 ( Esc )" onClick={close}>
          <X className="h-5 w-5" />
        </button>
      </div>

      <div className="flex min-h-0 flex-1">
        {/* Stage */}
        <div
          ref={stageRef}
          onWheel={onWheel}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          className="relative min-w-0 flex-1 cursor-grab overflow-hidden active:cursor-grabbing"
          style={{
            backgroundColor: "#1a1a1a",
            backgroundImage:
              "linear-gradient(45deg, #2a2a2a 25%, transparent 25%, transparent 75%, #2a2a2a 75%, #2a2a2a), linear-gradient(45deg, #2a2a2a 25%, transparent 25%, transparent 75%, #2a2a2a 75%, #2a2a2a)",
            backgroundSize: "24px 24px",
            backgroundPosition: "0 0, 12px 12px",
          }}
        >
          <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
            <div className="pointer-events-auto" style={{ transform: outerTransform }}>
              <img
                src={src}
                alt={title}
                draggable={false}
                onLoad={onImgLoad}
                className="block max-w-none select-none"
                style={{ transform: innerTransform, filter }}
              />
            </div>
          </div>

          <div className="pointer-events-none absolute bottom-3 left-3 rounded bg-black/50 px-2 py-1 font-mono text-[11px] text-white/70">
            {nat.w ? `${nat.w}×${nat.h}` : "…"} · {Math.round(scale * 100)}%
            {rot % 360 !== 0 ? ` · ${((rot % 360) + 360) % 360}°` : ""}
          </div>
        </div>

        {/* Adjustments */}
        <aside className="w-60 shrink-0 overflow-y-auto border-l border-white/10 bg-black/40 p-4 text-white">
          <div className="mb-3 flex items-center gap-1.5 text-xs font-semibold text-white/80">
            <SlidersHorizontal className="h-3.5 w-3.5" />
            图像调整
          </div>
          <div className="space-y-3.5">
            <AdjustSlider
              icon={Sun}
              label="亮度"
              value={adj.brightness}
              min={0}
              max={2}
              step={0.01}
              def={1}
              fmt={pct}
              onChange={(v) => setAdj((a) => ({ ...a, brightness: v }))}
            />
            <AdjustSlider
              icon={Contrast}
              label="对比度"
              value={adj.contrast}
              min={0}
              max={2}
              step={0.01}
              def={1}
              fmt={pct}
              onChange={(v) => setAdj((a) => ({ ...a, contrast: v }))}
            />
            <AdjustSlider
              icon={Aperture}
              label="曝光"
              value={adj.exposure}
              min={-1.5}
              max={1.5}
              step={0.05}
              def={0}
              fmt={(v) => (v > 0 ? `+${v.toFixed(2)}` : v.toFixed(2))}
              onChange={(v) => setAdj((a) => ({ ...a, exposure: v }))}
            />
            <AdjustSlider
              icon={Droplet}
              label="饱和度"
              value={adj.saturate}
              min={0}
              max={2}
              step={0.01}
              def={1}
              fmt={pct}
              onChange={(v) => setAdj((a) => ({ ...a, saturate: v }))}
            />
            <AdjustSlider
              icon={Sparkles}
              label="锐度"
              value={adj.sharpen}
              min={0}
              max={3}
              step={0.05}
              def={0}
              fmt={(v) => v.toFixed(2)}
              onChange={(v) => setAdj((a) => ({ ...a, sharpen: v }))}
            />
          </div>

          <button
            className="mt-4 w-full rounded border border-white/15 py-1.5 text-xs text-white/70 transition-colors hover:bg-white/10 hover:text-white"
            onClick={() => setAdj(DEFAULT_ADJUST)}
          >
            重置调整
          </button>

          <p className="mt-4 text-[10px] leading-relaxed text-white/40">
            滚轮缩放 · 拖动平移 · 空白处点击关闭。调整仅用于观察，不修改原图。
          </p>
        </aside>
      </div>
    </div>
  );
}

const pct = (v: number) => `${Math.round(v * 100)}%`;

function AdjustSlider({
  icon: Icon,
  label,
  value,
  min,
  max,
  step,
  def,
  fmt,
  onChange,
}: {
  icon: typeof Sun;
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  def: number;
  fmt: (v: number) => string;
  onChange: (v: number) => void;
}) {
  return (
    <div>
      <div className="mb-1 flex items-center justify-between text-[11px]">
        <span className="flex items-center gap-1 text-white/70">
          <Icon className="h-3 w-3" />
          {label}
        </span>
        <button
          className="font-mono text-white/50 transition-colors hover:text-white"
          title="双击数值重置"
          onClick={() => onChange(def)}
        >
          {fmt(value)}
        </button>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        onDoubleClick={() => onChange(def)}
        className="w-full accent-primary"
      />
    </div>
  );
}
