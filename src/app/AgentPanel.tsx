import { useEffect, useRef } from "react";
import { Check, Loader2, Sparkles, Square, X } from "lucide-react";

import { api } from "@/lib/bindings";
import { cn } from "@/lib/utils";
import { useAgentStore, type AgentStep } from "@/store/agent";

function stepColor(s: AgentStep): string {
  if (s.kind === "error" || s.ok === false) return "text-destructive";
  if (s.kind === "thinking") return "text-muted-foreground";
  if (s.kind === "done") return "text-green-600";
  if (s.kind === "result") return s.ok ? "text-green-600" : "text-destructive";
  return "text-foreground";
}

/** Floating, non-blocking panel that narrates the AI agent as it builds the graph
 * (rendered inside Canvas so it overlays the board). */
export function AgentPanel() {
  const running = useAgentStore((s) => s.running);
  const steps = useAgentStore((s) => s.steps);
  const job = useAgentStore((s) => s.job);
  const notes = useAgentStore((s) => s.notes);
  const error = useAgentStore((s) => s.error);
  const reset = useAgentStore((s) => s.reset);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    listRef.current?.scrollTo({ top: listRef.current.scrollHeight });
  }, [steps.length]);

  if (!running && steps.length === 0) return null;

  return (
    <div className="absolute bottom-4 right-4 z-30 flex max-h-[62%] w-[340px] flex-col rounded-lg border border-border bg-card shadow-2xl">
      <div className="flex items-center gap-2 border-b border-border px-3 py-2">
        <span className="flex h-6 w-6 items-center justify-center rounded bg-primary/10 text-primary">
          {running ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Sparkles className="h-3.5 w-3.5" />}
        </span>
        <span className="flex-1 text-sm font-medium">
          {running ? "AI 正在搭建流程…" : "搭建完成"}
        </span>
        {running && job ? (
          <button
            onClick={() => void api.cancelJob(job)}
            className="flex items-center gap-1 rounded border border-border px-1.5 py-0.5 text-[11px] text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
          >
            <Square className="h-2.5 w-2.5" /> 停止
          </button>
        ) : (
          <button
            onClick={reset}
            className="rounded p-0.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      <div ref={listRef} className="min-h-0 flex-1 space-y-0.5 overflow-y-auto px-2 py-2 text-[11px]">
        {steps.map((s, i) => (
          <div key={i} className={cn("flex items-start gap-1.5 leading-relaxed", stepColor(s))}>
            {s.kind === "done" && <Check className="mt-0.5 h-3 w-3 shrink-0" />}
            <span className="min-w-0 break-words font-mono">{s.text}</span>
          </div>
        ))}
      </div>

      {error && (
        <div className="border-t border-destructive/30 bg-destructive/10 px-3 py-1.5 text-[11px] text-destructive">
          {error}
        </div>
      )}
      {!running && notes && !error && (
        <div className="border-t border-border px-3 py-1.5 text-[11px] text-muted-foreground">
          {notes}
        </div>
      )}
    </div>
  );
}
