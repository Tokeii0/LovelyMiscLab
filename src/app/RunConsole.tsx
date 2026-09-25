import { cn } from "@/lib/utils";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useRunStore } from "@/store/run";

function statusColor(s: string) {
  return s === "done"
    ? "bg-green-500/15 text-green-600"
    : s === "running"
      ? "bg-blue-500/15 text-blue-600"
      : s === "error"
        ? "bg-red-500/15 text-red-600"
        : s === "stale"
          ? "bg-amber-500/15 text-amber-600"
          : "bg-secondary text-muted-foreground";
}

export function RunConsole() {
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const selectedId = useGraphStore((s) => s.selectedId);
  const byId = useDescriptorStore((s) => s.byId);
  const live = useRunStore((s) => s.mode === "live");
  const running = useRunStore((s) => s.running);
  const elapsed = useRunStore((s) => s.elapsed);
  const lastError = useRunStore((s) => s.lastError);
  const historyCount = useRunStore((s) => s.history.length);

  const errors = nodes.filter((n) => n.data.status === "error").length;
  const count = (f: (d: (typeof nodes)[number]["data"]) => boolean) =>
    nodes.filter((n) => f(n.data)).length;
  const counts: [string, string, number][] = [
    ["done", "完成", count((d) => d.status === "done" && !d.stale)],
    ["stale", "过期", count((d) => !!d.stale)],
    ["skipped", "跳过", count((d) => d.status === "skipped")],
    ["running", "运行中", count((d) => d.status === "running")],
  ];
  const selected = nodes.find((n) => n.id === selectedId);
  const modeText = running ? "运行中" : live ? "实时" : "就绪";

  const Stat = ({
    label,
    value,
    tone,
  }: {
    label: string;
    value: React.ReactNode;
    tone?: string;
  }) => (
    <span className="flex items-center gap-1 text-muted-foreground">
      {label}
      <b className={cn("font-semibold", tone ?? "text-foreground")}>{value}</b>
    </span>
  );

  return (
    <div className="flex h-10 shrink-0 items-center gap-4 border-t border-border bg-card px-3 text-[11px]">
      <span className="font-medium">运行控制台</span>
      <Stat label="节点" value={nodes.length} />
      <Stat label="连接" value={edges.length} />
      <Stat label="错误" value={errors} tone={errors ? "text-destructive" : "text-green-600"} />
      <Stat
        label="选中"
        value={selected ? (byId[selected.data.descriptorId]?.displayName ?? "—") : "—"}
      />
      <Stat
        label="状态"
        value={modeText}
        tone={running ? "text-blue-500" : live ? "text-emerald-600" : undefined}
      />
      <Stat label="耗时" value={`${(elapsed / 1000).toFixed(2)}s`} />
      <Stat label="历史" value={historyCount} />
      {lastError && (
        <span className="max-w-[360px] truncate text-destructive" title={lastError}>
          {lastError}
        </span>
      )}

      <div className="flex-1" />

      <div className="flex shrink-0 items-center gap-1">
        {counts.map(([key, label, n]) =>
          n > 0 ? (
            <span key={key} className={cn("whitespace-nowrap rounded px-1.5 py-0.5", statusColor(key))}>
              {label} {n}
            </span>
          ) : null
        )}
      </div>

      <span
        className="shrink-0 border-l border-border pl-3 font-mono text-muted-foreground/70"
        title="LovelyMiscLab 版本"
      >
        v{__APP_VERSION__}
      </span>
    </div>
  );
}
