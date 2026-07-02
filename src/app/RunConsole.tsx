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
        : "bg-secondary text-muted-foreground";
}

export function RunConsole() {
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const selectedId = useGraphStore((s) => s.selectedId);
  const byId = useDescriptorStore((s) => s.byId);
  const mode = useRunStore((s) => s.mode);
  const elapsed = useRunStore((s) => s.elapsed);
  const lastError = useRunStore((s) => s.lastError);
  const historyCount = useRunStore((s) => s.history.length);

  const errors = nodes.filter((n) => n.data.status === "error").length;
  const selected = nodes.find((n) => n.id === selectedId);
  const modeText = mode === "live" ? "实时" : mode === "paused" ? "暂停" : "手动";

  const Stat = ({ label, value, tone }: { label: string; value: React.ReactNode; tone?: string }) => (
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
      <Stat
        label="错误"
        value={errors}
        tone={errors ? "text-destructive" : "text-green-600"}
      />
      <Stat
        label="选中"
        value={selected ? byId[selected.data.descriptorId]?.displayName ?? "—" : "—"}
      />
      <Stat label="模式" value={modeText} />
      <Stat label="耗时" value={`${(elapsed / 1000).toFixed(2)}s`} />
      <Stat label="历史" value={historyCount} />
      {lastError && (
        <span className="max-w-[360px] truncate text-destructive" title={lastError}>
          {lastError}
        </span>
      )}

      <div className="flex-1" />

      <div className="flex min-w-0 items-center gap-1 overflow-x-auto">
        {nodes.slice(0, 10).map((n, i) => (
          <div key={n.id} className="flex items-center gap-1">
            {i > 0 && <span className="text-muted-foreground/50">→</span>}
            <span
              className={cn("whitespace-nowrap rounded px-1.5 py-0.5", statusColor(n.data.status))}
            >
              {byId[n.data.descriptorId]?.displayName ?? n.data.descriptorId}
            </span>
          </div>
        ))}
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
