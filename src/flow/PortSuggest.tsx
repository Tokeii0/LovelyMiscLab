import { useMemo, useState } from "react";
import { useReactFlow } from "@xyflow/react";
import { Loader2, Sparkles } from "lucide-react";

import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { NodeDescriptor } from "@/lib/types";
import { useGraphStore } from "@/store/graph";
import { usePortSuggest } from "@/store/portSuggest";

import { nodeIcon } from "./nodeIcons";
import { candidateNodes, firstCompatibleInput, firstCompatibleOutput, resolvePortType } from "./portUtils";

const WIDTH = 268;

// Remembered for the session: once we learn the LLM isn't configured, stop
// offering the AI-rank button (the static list still works fully offline).
let aiKnownUnavailable = false;

/** Port-hover panel: recommends a compatible next/previous node (static list +
 * optional AI ranking), supports manual filtering, and on pick creates the node
 * and wires it up. Rendered by Canvas so it has ReactFlow + store access. */
export function PortSuggest() {
  const ctx = usePortSuggest((s) => s.ctx);
  const close = usePortSuggest((s) => s.close);
  const rf = useReactFlow();

  const [query, setQuery] = useState("");
  const [aiOrder, setAiOrder] = useState<string[] | null>(null);
  const [aiReasons, setAiReasons] = useState<Record<string, string>>({});
  const [aiLoading, setAiLoading] = useState(false);
  const [aiError, setAiError] = useState("");

  const srcType = ctx ? resolvePortType(ctx.nodeId, ctx.port, ctx.dir) : undefined;

  const base = useMemo(
    () => (ctx && srcType ? candidateNodes(srcType, ctx.dir) : []),
    [ctx, srcType]
  );

  if (!ctx) return null;

  const q = query.trim().toLowerCase();
  const filtered = q
    ? base.filter(
        (d) => d.id.toLowerCase().includes(q) || d.displayName.toLowerCase().includes(q)
      )
    : base;
  // AI-ranked ids float to the top (in the model's order); the rest follow.
  const ordered = aiOrder
    ? [
        ...(aiOrder.map((id) => filtered.find((d) => d.id === id)).filter(Boolean) as NodeDescriptor[]),
        ...filtered.filter((d) => !aiOrder.includes(d.id)),
      ]
    : filtered;
  const display = ordered.slice(0, 12);

  const pick = (d: NodeDescriptor) => {
    const g = useGraphStore.getState();
    const src = g.nodes.find((n) => n.id === ctx.nodeId);
    if (!src || !srcType) {
      close();
      return;
    }
    const dx = ctx.dir === "out" ? 260 : -260;
    // Place beside the source, nudging down past any node already sitting there
    // so a suggested node never lands on top of an existing one.
    const x = src.position.x + dx;
    let y = src.position.y;
    const occupied = (yy: number) =>
      g.nodes.some((n) => Math.abs(n.position.x - x) < 60 && Math.abs(n.position.y - yy) < 60);
    while (occupied(y)) y += 90;
    const pos = { x, y };
    const newId = g.addNode(d, pos);
    if (ctx.dir === "out") {
      const match = firstCompatibleInput(d, srcType);
      if (match) {
        if (match.isParam) g.toggleParamInput(newId, match.port);
        g.onConnect({ source: ctx.nodeId, sourceHandle: ctx.port, target: newId, targetHandle: match.port });
      }
    } else {
      const outPort = firstCompatibleOutput(d, srcType);
      if (outPort) {
        g.onConnect({ source: newId, sourceHandle: outPort, target: ctx.nodeId, targetHandle: ctx.port });
      }
    }
    g.setSelected(newId);
    rf.setCenter(pos.x + 100, pos.y + 40, { zoom: rf.getViewport().zoom, duration: 250 });
    close();
  };

  const runAi = async () => {
    if (!srcType || aiLoading) return;
    setAiLoading(true);
    setAiError("");
    try {
      const res = await api.suggestNextNodes({
        descriptorId: ctx.descriptorId,
        port: ctx.port,
        direction: ctx.dir,
        portType: srcType,
        hint: query.trim() || undefined,
      });
      const reasons: Record<string, string> = {};
      for (const s of res) reasons[s.descriptorId] = s.reason;
      setAiReasons(reasons);
      setAiOrder(res.map((s) => s.descriptorId));
    } catch (e) {
      const msg = String(e);
      if (msg.includes("未配置")) aiKnownUnavailable = true;
      setAiError(msg);
    } finally {
      setAiLoading(false);
    }
  };

  const left = Math.min(ctx.anchor.x + 8, window.innerWidth - WIDTH - 8);
  const top = Math.min(ctx.anchor.y, window.innerHeight - 380);
  const title = ctx.dir === "out" ? "接下一个节点" : "接上一个节点";

  return (
    <>
      <div
        className="fixed inset-0 z-[60]"
        onClick={close}
        onContextMenu={(e) => {
          e.preventDefault();
          close();
        }}
      />
      <div
        className="fixed z-[61] flex max-h-[368px] flex-col rounded-lg border border-border bg-popover text-xs shadow-xl"
        style={{ left, top, width: WIDTH }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-1.5 border-b border-border px-2.5 py-2">
          <Sparkles className="h-3.5 w-3.5 text-primary" />
          <span className="font-medium">{title}</span>
          <span className="ml-auto rounded bg-secondary px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
            {srcType ?? "?"}
          </span>
        </div>

        <div className="p-2">
          <input
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && display[0]) pick(display[0]);
              else if (e.key === "Escape") close();
            }}
            placeholder="筛选 / 描述想做的事…"
            className="w-full rounded border border-input bg-background px-2 py-1 focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-1 pb-1">
          {display.length === 0 && (
            <div className="px-2 py-3 text-center text-muted-foreground">无兼容节点</div>
          )}
          {display.map((d) => {
            const Icon = nodeIcon(d.id, d.category);
            const reason = aiReasons[d.id];
            return (
              <button
                key={d.id}
                onClick={() => pick(d)}
                className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left transition-colors hover:bg-accent"
              >
                <span
                  className="flex h-5 w-5 shrink-0 items-center justify-center rounded"
                  style={{ background: `${d.color}18`, color: d.color }}
                >
                  <Icon className="h-3 w-3" />
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate font-medium">{d.displayName}</span>
                  <span className="block truncate text-[10px] text-muted-foreground">
                    {reason || d.category}
                  </span>
                </span>
                {reason && <Sparkles className="h-3 w-3 shrink-0 text-primary/70" />}
              </button>
            );
          })}
        </div>

        <div className="flex items-center justify-between gap-2 border-t border-border px-2 py-1.5">
          {inTauri && !aiKnownUnavailable ? (
            <button
              onClick={runAi}
              disabled={aiLoading}
              className="flex items-center gap-1 rounded px-1.5 py-1 text-[11px] text-primary transition-colors hover:bg-primary/10 disabled:opacity-50"
            >
              {aiLoading ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                <Sparkles className="h-3 w-3" />
              )}
              AI 推荐排序
            </button>
          ) : (
            <span className="text-[10px] text-muted-foreground">桌面版可用 AI 排序</span>
          )}
          <span className="text-[10px] text-muted-foreground">Enter 采用首项</span>
        </div>
        {aiError && (
          <div className="border-t border-destructive/30 bg-destructive/10 px-2 py-1 text-[10px] text-destructive">
            {aiError.includes("未配置") ? "AI 未配置，可在设置中填写文本模型" : aiError}
          </div>
        )}
      </div>
    </>
  );
}
