import type { Edge } from "@xyflow/react";

import type { FlowNode } from "@/store/graph";

export interface LayoutOptions {
  /** Horizontal distance between columns (node width + gap). */
  xGap?: number;
  /** Vertical gap between stacked nodes in the same column. */
  yGap?: number;
  x0?: number;
  y0?: number;
}

/** Longest-path depth per node (0 for roots / isolated). Bounded relaxation so a
 * cycle can never spin forever — cycle members settle at the deepest level. */
export function longestPathLevels(nodes: FlowNode[], edges: Edge[]): Record<string, number> {
  const ids = new Set(nodes.map((n) => n.id));
  const rel = edges.filter((e) => ids.has(e.source) && ids.has(e.target));
  const level: Record<string, number> = {};
  for (const n of nodes) level[n.id] = 0;
  for (let i = 0; i < nodes.length; i++) {
    let changed = false;
    for (const e of rel) {
      if (level[e.source] + 1 > level[e.target]) {
        level[e.target] = level[e.source] + 1;
        changed = true;
      }
    }
    if (!changed) break;
  }
  return level;
}

/** Aspect ratio (w/h) of the visible canvas, used to pack the graph so a
 * following fitView shows every node as large as possible. Landscape fallback. */
export function viewportAspect(): number {
  const el = typeof document !== "undefined" ? document.querySelector(".react-flow") : null;
  if (el instanceof HTMLElement && el.clientHeight > 0) return el.clientWidth / el.clientHeight;
  return 1.4;
}

/**
 * Pack nodes so they all fit the current view: fill each column top→bottom
 * (vertical first), then wrap to the next column (horizontal) once it's full —
 * this is the "先垂直再水平" arrangement. The column count is chosen so the whole
 * block matches the viewport's aspect ratio, so a following `fitView` shows
 * everything at a comfortable zoom instead of a long horizontal strip that
 * forces a tiny zoom. Fill order follows dependency depth so the flow still
 * reads upstream→downstream, and measured node heights prevent overlap.
 */
export function packedLayout(
  nodes: FlowNode[],
  edges: Edge[],
  aspect: number,
  opts: LayoutOptions = {}
): Record<string, { x: number; y: number }> {
  const colStep = opts.xGap ?? 260;
  const yGap = opts.yGap ?? 40;
  const x0 = opts.x0 ?? 40;
  const y0 = opts.y0 ?? 40;

  const positions: Record<string, { x: number; y: number }> = {};
  if (nodes.length === 0) return positions;

  // Fill in execution order (depth, then current array order) so a column reads
  // upstream→downstream.
  const level = longestPathLevels(nodes, edges);
  const idx = new Map(nodes.map((n, i) => [n.id, i]));
  const ordered = [...nodes].sort(
    (a, b) => level[a.id] - level[b.id] || (idx.get(a.id) ?? 0) - (idx.get(b.id) ?? 0)
  );

  const heights = ordered.map((n) => (n.measured?.height ?? 120) + yGap);
  const totalH = heights.reduce((s, h) => s + h, 0);
  // cols so (cols·colStep) / (totalH/cols) ≈ aspect  →  cols ≈ √(aspect·totalH/colStep).
  const cols = Math.max(1, Math.round(Math.sqrt((Math.max(aspect, 0.2) * totalH) / colStep)));
  const targetH = totalH / cols;

  let col = 0;
  let y = y0;
  for (let i = 0; i < ordered.length; i++) {
    const h = heights[i];
    // Wrap to a fresh column once this one is full (never on the last column, so
    // no node is orphaned past the grid).
    if (y > y0 && y - y0 + h > targetH && col < cols - 1) {
      col++;
      y = y0;
    }
    positions[ordered[i].id] = { x: x0 + col * colStep, y };
    y += h;
  }
  return positions;
}
