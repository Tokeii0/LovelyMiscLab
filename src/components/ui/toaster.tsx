import { CheckCircle2, Info, X, XCircle } from "lucide-react";

import { cn } from "@/lib/utils";
import { useToastStore, type ToastKind } from "@/store/toast";

const ICON: Record<ToastKind, typeof Info> = {
  success: CheckCircle2,
  info: Info,
  error: XCircle,
};

const TINT: Record<ToastKind, string> = {
  success: "text-emerald-500",
  info: "text-primary",
  error: "text-destructive",
};

/** Bottom-right notification stack; mounted once in App. */
export function Toaster() {
  const items = useToastStore((s) => s.items);
  const dismiss = useToastStore((s) => s.dismiss);
  if (items.length === 0) return null;

  return (
    <div className="pointer-events-none fixed bottom-12 right-4 z-[110] flex w-[360px] max-w-[calc(100vw-2rem)] flex-col gap-2">
      {items.map((t) => {
        const Icon = ICON[t.kind];
        return (
          <div
            key={t.id}
            role={t.kind === "error" ? "alert" : "status"}
            className="pointer-events-auto flex items-start gap-2 rounded-lg border border-border bg-card px-3 py-2.5 text-sm shadow-xl"
          >
            <Icon className={cn("mt-0.5 h-4 w-4 shrink-0", TINT[t.kind])} />
            <div className="min-w-0 flex-1">
              <div className="font-medium">{t.message}</div>
              {t.detail && (
                <div className="mt-0.5 line-clamp-4 select-text break-words text-xs text-muted-foreground">
                  {t.detail}
                </div>
              )}
              {t.actions.length > 0 && (
                <div className="mt-2 flex gap-2">
                  {t.actions.map((a) => (
                    <button
                      key={a.label}
                      onClick={() => {
                        dismiss(t.id);
                        a.run();
                      }}
                      className="rounded-md border border-border px-2 py-0.5 text-xs font-medium hover:bg-accent"
                    >
                      {a.label}
                    </button>
                  ))}
                </div>
              )}
            </div>
            <button
              onClick={() => dismiss(t.id)}
              title="关闭"
              className="rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        );
      })}
    </div>
  );
}
