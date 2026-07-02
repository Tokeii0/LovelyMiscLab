import { useMemo, useState } from "react";

import { cn } from "@/lib/utils";
import { TEMPLATES, TEMPLATE_CATEGORIES, type Template } from "@/lib/templates";
import { loadTemplate } from "@/flow/loadTemplate";
import { useDescriptorStore } from "@/store/descriptors";
import { useViewStore } from "@/store/view";

/** A compact, fully-clickable template card. */
function Card({ t, onUse }: { t: Template; onUse: (t: Template) => void }) {
  const byId = useDescriptorStore((s) => s.byId);
  const Icon = t.icon;
  const chain = t.nodes
    .map((n) => byId[n.descriptorId]?.displayName ?? n.descriptorId)
    .join(" → ");
  return (
    <button
      onClick={() => onUse(t)}
      title={chain}
      className="group flex flex-col gap-1.5 rounded-lg border border-border bg-card p-2.5 text-left transition-colors hover:border-primary hover:bg-accent/40"
    >
      <div className="flex w-full items-center gap-2">
        <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
          <Icon className="h-4 w-4" />
        </span>
        <span className="min-w-0 flex-1 truncate text-[13px] font-medium leading-tight">
          {t.name}
        </span>
        <span className="shrink-0 rounded bg-secondary px-1.5 py-0.5 text-[10px] text-muted-foreground">
          {t.nodes.length}
        </span>
      </div>
      <p className="line-clamp-2 text-[11px] leading-snug text-muted-foreground">
        {t.description}
      </p>
    </button>
  );
}

export function TemplatesView() {
  const setView = useViewStore((s) => s.setView);
  const [cat, setCat] = useState<string>("全部");
  const cats = ["全部", ...TEMPLATE_CATEGORIES];

  // Group templates by category (in declared order), filtered by the active pill.
  const sections = useMemo(
    () =>
      TEMPLATE_CATEGORIES.map(
        (c) => [c, TEMPLATES.filter((t) => t.category === c)] as const
      ).filter(([c, items]) => items.length > 0 && (cat === "全部" || cat === c)),
    [cat]
  );

  const use = (t: Template) => {
    loadTemplate(t);
    setView("canvas");
  };

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-4 py-3">
        <h1 className="text-base font-semibold">流程模板</h1>
        <p className="text-[11px] text-muted-foreground">
          内置常见 CTF Misc 解题流程，一键载入画布即可运行或在其上改造。
        </p>
        <div className="mt-2.5 flex flex-wrap gap-1">
          {cats.map((c) => (
            <button
              key={c}
              onClick={() => setCat(c)}
              className={cn(
                "rounded-full px-2.5 py-0.5 text-[11px] transition-colors",
                cat === c
                  ? "bg-primary text-primary-foreground"
                  : "bg-secondary text-muted-foreground hover:bg-accent hover:text-foreground"
              )}
            >
              {c}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 space-y-5 overflow-y-auto p-4">
        {sections.map(([category, items]) => (
          <section key={category}>
            <h2 className="mb-2 flex items-center gap-2 text-xs font-semibold text-foreground/80">
              {category}
              <span className="text-[10px] font-normal text-muted-foreground">
                {items.length}
              </span>
            </h2>
            <div className="grid grid-cols-2 gap-2.5 md:grid-cols-3 xl:grid-cols-4">
              {items.map((t) => (
                <Card key={t.id} t={t} onUse={use} />
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
