import { useMemo, useState } from "react";
import { ArrowRight } from "lucide-react";

import { cn } from "@/lib/utils";
import { TEMPLATES, TEMPLATE_CATEGORIES, type Template } from "@/lib/templates";
import { loadTemplate } from "@/flow/loadTemplate";
import { useDescriptorStore } from "@/store/descriptors";
import { useViewStore } from "@/store/view";

function Chain({ t }: { t: Template }) {
  const byId = useDescriptorStore((s) => s.byId);
  const names = t.nodes.map((n) => byId[n.descriptorId]?.displayName ?? n.descriptorId);
  return (
    <div className="flex flex-wrap items-center gap-1">
      {names.map((name, i) => (
        <span key={i} className="flex items-center gap-1">
          <span className="rounded bg-background px-1.5 py-0.5 text-[10px] text-muted-foreground">
            {name}
          </span>
          {i < names.length - 1 && (
            <ArrowRight className="h-3 w-3 text-muted-foreground/50" />
          )}
        </span>
      ))}
    </div>
  );
}

export function TemplatesView() {
  const setView = useViewStore((s) => s.setView);
  const [cat, setCat] = useState<string>("全部");
  const cats = ["全部", ...TEMPLATE_CATEGORIES];
  const list = useMemo(
    () => TEMPLATES.filter((t) => cat === "全部" || t.category === cat),
    [cat]
  );

  const use = (t: Template) => {
    loadTemplate(t);
    setView("canvas");
  };

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border p-4">
        <h1 className="text-lg font-semibold">流程模板</h1>
        <p className="text-xs text-muted-foreground">
          内置常见 CTF Misc 解题流程，一键载入画布即可运行或在其上改造。
        </p>
        <div className="mt-3 flex flex-wrap gap-1">
          {cats.map((c) => (
            <button
              key={c}
              onClick={() => setCat(c)}
              className={cn(
                "rounded-full px-3 py-1 text-xs transition-colors",
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

      <div className="flex-1 overflow-y-auto p-4">
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
          {list.map((t) => {
            const Icon = t.icon;
            return (
              <div
                key={t.id}
                className="flex flex-col rounded-xl border border-border bg-card p-4 transition-all hover:border-primary hover:shadow-md"
              >
                <div className="flex items-start gap-2">
                  <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                    <Icon className="h-5 w-5" />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="text-sm font-semibold">{t.name}</div>
                    <div className="text-[10px] text-muted-foreground">
                      {t.category} · {t.nodes.length} 节点
                    </div>
                  </div>
                </div>
                <p className="mt-2 flex-1 text-xs leading-relaxed text-muted-foreground">
                  {t.description}
                </p>
                <div className="mt-3 rounded-lg bg-secondary/40 p-2">
                  <Chain t={t} />
                </div>
                <button
                  onClick={() => use(t)}
                  className="mt-3 rounded-md bg-primary py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
                >
                  使用模板
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
