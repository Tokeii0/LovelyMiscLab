import { Cog, MessageCircle } from "lucide-react";

import type { GalgameChoice } from "@/lib/bindings";
import { cn } from "@/lib/utils";

/** The 2–4 branch choices. A choice with a `node` runs one real solving node
 * (marked with a gear); others just advance the story. Clicking reports the
 * pointer position so the view can fire a particle burst there.
 *
 * Colors are pinned to a dark-glass look with light text (NOT theme tokens):
 * the galgame scene is always dark, so theme-flipping `bg-card`/`text-foreground`
 * would make hover text dark-on-dark in light mode. */
export function ChoiceList({
  choices,
  disabled,
  onPick,
}: {
  choices: GalgameChoice[];
  disabled?: boolean;
  onPick: (c: GalgameChoice, at?: { x: number; y: number }) => void;
}) {
  return (
    <div className="pointer-events-auto flex flex-col gap-2">
      {choices.map((c, i) => {
        const solving = !!c.node;
        return (
          <button
            key={`${i}-${c.text}`}
            disabled={disabled}
            onClick={(e) => onPick(c, { x: e.clientX, y: e.clientY })}
            className={cn(
              "group relative flex items-center gap-3 overflow-hidden rounded-xl border px-4 py-3 text-left text-sm text-slate-100 backdrop-blur-md transition-all duration-200",
              "bg-slate-900/60 hover:bg-slate-800/80 hover:shadow-[0_0_26px_-6px_rgba(59,130,246,0.7)]",
              "disabled:cursor-not-allowed disabled:opacity-50",
              solving ? "border-primary/45 hover:border-primary/80" : "border-white/10 hover:border-primary/60"
            )}
          >
            {/* left accent bar grows in on hover */}
            <span className="absolute inset-y-1 left-0 w-1 origin-center scale-y-0 rounded-r bg-primary transition-transform duration-200 group-hover:scale-y-100" />
            {/* diagonal sheen sweep on hover */}
            <span className="pointer-events-none absolute inset-0 -translate-x-full bg-gradient-to-r from-transparent via-white/15 to-transparent transition-transform duration-500 ease-out group-hover:translate-x-full" />

            <span
              className={cn(
                "relative flex h-7 w-7 shrink-0 items-center justify-center rounded-lg transition-colors",
                solving
                  ? "bg-primary/25 text-sky-200 group-hover:bg-primary group-hover:text-white"
                  : "bg-white/10 text-slate-300 group-hover:bg-white/20 group-hover:text-white"
              )}
            >
              {solving ? <Cog className="h-4 w-4" /> : <MessageCircle className="h-4 w-4" />}
            </span>
            <span className="relative flex-1">{c.text}</span>
            {solving && (
              <span className="relative shrink-0 rounded-md bg-primary/20 px-2 py-0.5 text-[10px] font-medium text-sky-200">
                解题
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
