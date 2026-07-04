import { cn } from "@/lib/utils";

/** A reusable progress-bar control: a filled track plus an optional status line
 * (left) and percentage (right). `value` is a 0..1 fraction. Used on nodes to
 * monitor long-running work (e.g. bkcrack) and in the inspector. */
export function ProgressBar({
  value,
  status,
  showLabel = true,
  className,
  barClassName,
}: {
  value: number;
  /** Optional short status text shown on the left (e.g. current stage). */
  status?: string;
  /** Show the "NN%" label on the right. */
  showLabel?: boolean;
  className?: string;
  barClassName?: string;
}) {
  const pct = Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));
  const p = Math.round(pct * 100);
  const indeterminate = pct === 0;
  return (
    <div className={cn("space-y-0.5", className)}>
      <div className="relative h-1.5 w-full overflow-hidden rounded-full bg-secondary">
        <div
          className={cn(
            "h-full rounded-full bg-primary transition-[width] duration-200",
            indeterminate && "animate-pulse",
            barClassName
          )}
          style={{ width: indeterminate ? "35%" : `${p}%` }}
        />
      </div>
      {(showLabel || status) && (
        <div className="flex items-center justify-between gap-2 text-[9px] leading-none text-muted-foreground">
          <span className="min-w-0 truncate" title={status}>
            {status ?? ""}
          </span>
          {showLabel && <span className="shrink-0 tabular-nums">{p}%</span>}
        </div>
      )}
    </div>
  );
}
