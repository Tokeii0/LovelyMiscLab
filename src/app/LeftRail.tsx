import {
  Boxes,
  History,
  LayoutGrid,
  type LucideIcon,
  Package,
  Settings,
  Workflow,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { useViewStore, type View } from "@/store/view";

const ITEMS: { view: View; label: string; icon: LucideIcon }[] = [
  { view: "canvas", label: "画布", icon: LayoutGrid },
  { view: "modules", label: "模块", icon: Boxes },
  { view: "templates", label: "模板", icon: Workflow },
  { view: "runs", label: "运行记录", icon: History },
  { view: "resources", label: "资源", icon: Package },
  { view: "settings", label: "设置", icon: Settings },
];

export function LeftRail() {
  const view = useViewStore((s) => s.view);
  const setView = useViewStore((s) => s.setView);

  return (
    <div className="flex w-16 shrink-0 flex-col items-center gap-1 border-r border-border bg-card py-2">
      {ITEMS.map(({ view: v, label, icon: Icon }) => {
        const active = view === v;
        return (
          <button
            key={v}
            onClick={() => setView(v)}
            className={cn(
              "relative flex w-14 flex-col items-center gap-1 rounded-lg py-2 text-[10px] transition-colors",
              active
                ? "bg-primary/10 font-medium text-primary"
                : "text-muted-foreground hover:bg-accent hover:text-foreground"
            )}
          >
            {active && (
              <span className="absolute left-0 top-1/2 h-6 w-0.5 -translate-y-1/2 rounded-r bg-primary" />
            )}
            <Icon className="h-5 w-5" />
            {label}
          </button>
        );
      })}
      <div className="flex-1" />
      <div className="text-[9px] text-muted-foreground">本地模式</div>
    </div>
  );
}
