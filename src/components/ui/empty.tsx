import type { LucideIcon } from "lucide-react";

/** Centered placeholder for an empty page or list. */
export function Empty({
  icon: Icon,
  title,
  hint,
}: {
  icon: LucideIcon;
  title: string;
  hint: string;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
      <span className="flex h-14 w-14 items-center justify-center rounded-lg bg-secondary text-muted-foreground">
        <Icon className="h-7 w-7" />
      </span>
      <div className="text-sm font-medium text-foreground">{title}</div>
      <div className="max-w-xs text-xs text-muted-foreground">{hint}</div>
    </div>
  );
}
