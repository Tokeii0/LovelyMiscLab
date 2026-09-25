// Where new nodes land. Every way of adding a node (library click, palette,
// modules page, paste, suggestions) goes through here, so nodes appear inside
// the visible canvas and never stack exactly on top of an existing one.

import type { ReactFlowInstance } from "@xyflow/react";

import { useGraphStore, type FlowNode } from "@/store/graph";

const DEFAULT_W = 200;
const DEFAULT_H = 96;
const GAP = 16;

let flow: ReactFlowInstance | null = null;

/** Called by <Canvas> so non-React code can map screen ↔ flow coordinates. */
export function registerFlow(rf: ReactFlowInstance | null) {
  flow = rf;
}

type Point = { x: number; y: number };

function rectOf(n: FlowNode) {
  return {
    x: n.position.x,
    y: n.position.y,
    w: n.measured?.width ?? n.width ?? DEFAULT_W,
    h: n.measured?.height ?? n.height ?? DEFAULT_H,
  };
}

function overlaps(p: Point, nodes: FlowNode[]): boolean {
  return nodes.some((n) => {
    const r = rectOf(n);
    return (
      p.x < r.x + r.w + GAP &&
      p.x + DEFAULT_W + GAP > r.x &&
      p.y < r.y + r.h + GAP &&
      p.y + DEFAULT_H + GAP > r.y
    );
  });
}

/** The first spot at/near `preferred` (flow coords) not covered by another node:
 * walks down in small steps, then over to the next column. */
export function freePosition(preferred: Point, nodes: FlowNode[] = useGraphStore.getState().nodes): Point {
  for (let col = 0; col < 8; col++) {
    for (let row = 0; row < 12; row++) {
      const p = { x: preferred.x + col * (DEFAULT_W + 2 * GAP), y: preferred.y + row * 48 };
      if (!overlaps(p, nodes)) return p;
    }
  }
  return preferred;
}

/** Center of the visible canvas in flow coordinates. */
export function viewportCenter(): Point {
  const el = document.querySelector<HTMLElement>(".react-flow");
  if (!flow || !el) return { x: 240, y: 160 };
  const r = el.getBoundingClientRect();
  return flow.screenToFlowPosition({ x: r.left + r.width / 2, y: r.top + r.height / 2 });
}

/** A free spot around the middle of what the user is looking at. */
export function placeInView(): Point {
  const c = viewportCenter();
  return freePosition({ x: c.x - DEFAULT_W / 2, y: c.y - DEFAULT_H / 2 });
}

/** Flow position under a screen point (e.g. the mouse), or the view center. */
export function screenToFlow(p: Point | null): Point {
  if (!flow || !p) return viewportCenter();
  return flow.screenToFlowPosition(p);
}
