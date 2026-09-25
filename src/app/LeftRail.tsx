import {
  Boxes,
  Gamepad2,
  History,
  LayoutGrid,
  type LucideIcon,
  Package,
  Settings,
  Workflow,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { usePrefs } from "@/store/prefs";
import { useViewStore, VIEW_LABEL, type View } from "@/store/view";

type Item = { view: View; label: string; icon: LucideIcon };

const ITEMS: Item[] = [
  { view: "canvas", label: VIEW_LABEL.canvas, icon: LayoutGrid },
  { view: "modules", label: VIEW_LABEL.modules, icon: Boxes },
  { view: "templates", label: VIEW_LABEL.templates, icon: Workflow },
  { view: "runs", label: VIEW_LABEL.runs, icon: History },
  { view: "resources", label: VIEW_LABEL.resources, icon: Package },
  { view: "galgame", label: VIEW_LABEL.galgame, icon: Gamepad2 },
];
const SETTINGS: Item = { view: "settings", label: VIEW_LABEL.settings, icon: Settings };

export function LeftRail() {
  const view = useViewStore((s) => s.view);
  const setView = useViewStore((s) => s.setView);
  const galgame = usePrefs((s) => s.experimentalGalgame);
  const items = ITEMS.filter((i) => i.view !== "galgame" || galgame || view === "galgame");

  const renderItem = ({ view: v, label, icon: Icon }: Item) => {
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
  };

  return (
    <div className="flex w-16 shrink-0 flex-col items-center gap-1 border-r border-border bg-card py-2">
      {items.map(renderItem)}
      <div className="flex-1" />
      {renderItem(SETTINGS)}
    </div>
  );
}
