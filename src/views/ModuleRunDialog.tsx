import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Check, Clock, Copy, Inbox, Play, RotateCcw, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { NodeDescriptor, ParamSpec, PortValue } from "@/lib/types";
import { cn } from "@/lib/utils";
import { nodeIcon } from "@/flow/nodeIcons";

// ---- helpers ---------------------------------------------------------------

function outText(v: PortValue): string {
  switch (v.type) {
    case "text":
      return v.value;
    case "number":
      return String(v.value);
    case "bool":
      return v.value ? "true" : "false";
    case "stringList":
      return v.value.join("\n");
    case "candidates":
      return v.value.map((c) => `${c.score.toFixed(2)}  ${c.text}`).join("\n");
    case "bytes":
      return `<${v.value.length} 字节>`;
    default:
      return JSON.stringify((v as { value?: unknown }).value ?? "", null, 2);
  }
}

const byteLen = (s: string) => new TextEncoder().encode(s).length;

function outputSize(outputs: Record<string, PortValue>): number {
  let n = 0;
  for (const v of Object.values(outputs)) {
    if (v.type === "text" || v.type === "image" || v.type === "artifact") n += byteLen(v.value);
    else if (v.type === "bytes") n += v.value.length;
    else if (v.type === "stringList") n += byteLen(v.value.join(""));
    else n += byteLen(JSON.stringify((v as { value?: unknown }).value ?? ""));
  }
  return n;
}

const isFormatSelect = (p: ParamSpec) =>
  p.widget.kind === "select" && /format|格式/i.test(`${p.name} ${p.label}`);

type Row = { kind: "single"; p: ParamSpec } | { kind: "pair"; a: ParamSpec; b: ParamSpec };

/** Lay params into rows: pair a value field with its trailing *format* select, or
 * two consecutive format selects, else full-width singles. */
function groupParams(params: ParamSpec[]): Row[] {
  const rows: Row[] = [];
  for (let i = 0; i < params.length; i++) {
    const p = params[i];
    const next = params[i + 1];
    const isValue = p.widget.kind === "text" || p.widget.kind === "number";
    if (isValue && next && isFormatSelect(next)) {
      rows.push({ kind: "pair", a: p, b: next });
      i++;
    } else if (isFormatSelect(p) && next && isFormatSelect(next)) {
      rows.push({ kind: "pair", a: p, b: next });
      i++;
    } else {
      rows.push({ kind: "single", p });
    }
  }
  return rows;
}

const inputCls =
  "w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring";

function Segmented({
  options,
  value,
  onChange,
}: {
  options: string[];
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="flex gap-1 rounded-lg bg-secondary p-1">
      {options.map((o) => (
        <button
          key={o}
          onClick={() => onChange(o)}
          className={cn(
            "flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-colors",
            value === o
              ? "bg-primary text-primary-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground"
          )}
        >
          {o}
        </button>
      ))}
    </div>
  );
}

function CopyBtn({ text }: { text: string }) {
  const [done, setDone] = useState(false);
  return (
    <button
      onClick={() => {
        navigator.clipboard?.writeText(text).catch(() => {});
        setDone(true);
        setTimeout(() => setDone(false), 1200);
      }}
      className="flex items-center gap-1 rounded border border-border px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-accent"
    >
      {done ? <Check className="h-3 w-3 text-green-600" /> : <Copy className="h-3 w-3" />}
      复制
    </button>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2 text-sm font-semibold">
      <span className="h-4 w-1 rounded-full bg-primary" />
      {children}
    </div>
  );
}

function EmptyBox({ text }: { text: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 py-12 text-center">
      <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-secondary text-muted-foreground/50">
        <Inbox className="h-7 w-7" />
      </span>
      <span className="text-xs text-muted-foreground">{text}</span>
    </div>
  );
}

interface HistoryEntry {
  time: string;
  params: Record<string, unknown>;
  inputs: Record<string, string>;
  outputs: Record<string, PortValue> | null;
  error: string;
  elapsed: number;
}

// ---- component -------------------------------------------------------------

export function ModuleRunDialog({
  descriptor,
  onClose,
}: {
  descriptor: NodeDescriptor;
  onClose: () => void;
}) {
  const [inputs, setInputs] = useState<Record<string, string>>({});
  const [params, setParams] = useState<Record<string, unknown>>(() =>
    Object.fromEntries(descriptor.params.map((p) => [p.name, p.default]))
  );
  const [outputs, setOutputs] = useState<Record<string, PortValue> | null>(null);
  const [error, setError] = useState("");
  const [running, setRunning] = useState(false);
  const [elapsed, setElapsed] = useState<number | null>(null);
  const [tab, setTab] = useState<"result" | "logs" | "history">("result");
  const [logs, setLogs] = useState<{ time: string; level: string; message: string }[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);

  const Icon = nodeIcon(descriptor.id, descriptor.category);
  const rows = useMemo(() => groupParams(descriptor.params), [descriptor]);
  const setP = (name: string, v: unknown) => setParams((pp) => ({ ...pp, [name]: v }));

  const status = running
    ? { t: "运行中", c: "#3b82f6" }
    : error
      ? { t: "失败", c: "#ef4444" }
      : outputs
        ? { t: "成功", c: "#22c55e" }
        : { t: "待运行", c: "#94a3b8" };

  const run = async () => {
    if (!inTauri) {
      setError("浏览器预览无法执行模块，请在应用中运行。");
      setTab("result");
      return;
    }
    setRunning(true);
    setError("");
    setOutputs(null);
    const t0 = Date.now();
    const time = new Date().toLocaleTimeString();
    setLogs((l) => [...l, { time, level: "info", message: "开始执行" }]);
    const inputMap: Record<string, PortValue> = {};
    for (const port of descriptor.inputs) {
      inputMap[port.name] = { type: "text", value: inputs[port.name] ?? "" };
    }
    try {
      const out = await api.runNode(descriptor.id, inputMap, params);
      const el = Date.now() - t0;
      setOutputs(out);
      setElapsed(el);
      setLogs((l) => [
        ...l,
        { time: new Date().toLocaleTimeString(), level: "success", message: `执行成功（${el}ms）` },
      ]);
      setHistory((h) =>
        [{ time, params: { ...params }, inputs: { ...inputs }, outputs: out, error: "", elapsed: el }, ...h].slice(0, 20)
      );
    } catch (e) {
      const el = Date.now() - t0;
      setError(String(e));
      setElapsed(el);
      setTab("result");
      setLogs((l) => [...l, { time: new Date().toLocaleTimeString(), level: "error", message: String(e) }]);
      setHistory((h) =>
        [{ time, params: { ...params }, inputs: { ...inputs }, outputs: null, error: String(e), elapsed: el }, ...h].slice(0, 20)
      );
    } finally {
      setRunning(false);
    }
  };

  const reset = () => {
    setParams(Object.fromEntries(descriptor.params.map((p) => [p.name, p.default])));
    setInputs({});
  };

  const restore = (h: HistoryEntry) => {
    setParams({ ...h.params });
    setInputs({ ...h.inputs });
    setOutputs(h.outputs);
    setError(h.error);
    setElapsed(h.elapsed);
    setTab("result");
  };

  const control = (p: ParamSpec) => {
    const w = p.widget;
    const value = params[p.name];
    switch (w.kind) {
      case "select":
        if (isFormatSelect(p) || w.options.length > 5) {
          return (
            <select value={String(value ?? "")} onChange={(e) => setP(p.name, e.target.value)} className={inputCls}>
              {w.options.map((o) => (
                <option key={o} value={o}>
                  {o}
                </option>
              ))}
            </select>
          );
        }
        return <Segmented options={w.options} value={String(value ?? "")} onChange={(v) => setP(p.name, v)} />;
      case "toggle":
        return (
          <button
            onClick={() => setP(p.name, !value)}
            className={cn(
              "relative inline-flex h-5 w-9 items-center rounded-full transition-colors",
              value ? "bg-primary" : "bg-secondary"
            )}
          >
            <span
              className={cn(
                "inline-block h-4 w-4 rounded-full bg-white shadow transition-transform",
                value ? "translate-x-4" : "translate-x-0.5"
              )}
            />
          </button>
        );
      case "number":
      case "slider":
        return (
          <input
            type="number"
            value={Number(value ?? 0)}
            min={w.min}
            max={w.max}
            step={w.step}
            onChange={(e) => setP(p.name, parseFloat(e.target.value))}
            className={inputCls}
          />
        );
      case "file": {
        const path = typeof value === "string" ? value : "";
        return (
          <div className="flex items-center gap-2">
            <button
              onClick={async () => {
                if (!inTauri) return;
                const sel = await open({ multiple: false, directory: false });
                if (typeof sel === "string") setP(p.name, sel);
              }}
              className="rounded-lg border border-input bg-background px-3 py-2 text-xs hover:bg-accent"
            >
              选择文件
            </button>
            <span className="truncate text-xs text-muted-foreground">{path ? path.split(/[\\/]/).pop() : "未选择"}</span>
          </div>
        );
      }
      default:
        return w.multiline ? (
          <textarea
            rows={2}
            value={String(value ?? "")}
            onChange={(e) => setP(p.name, e.target.value)}
            className={cn(inputCls, "resize-y")}
          />
        ) : (
          <input
            value={String(value ?? "")}
            onChange={(e) => setP(p.name, e.target.value)}
            placeholder={`请输入${p.label}`}
            className={inputCls}
          />
        );
    }
  };

  const Field = ({ p }: { p: ParamSpec }) => (
    <div>
      <label className="mb-1 block text-[11px] font-medium text-muted-foreground">{p.label}</label>
      {control(p)}
    </div>
  );

  return (
    <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        className="flex max-h-[88vh] w-[1080px] max-w-[95vw] flex-col overflow-hidden rounded-xl border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* header */}
        <div className="flex items-center gap-3 border-b border-border px-5 py-4">
          <span
            className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl"
            style={{ background: `${descriptor.color}18`, color: descriptor.color }}
          >
            <Icon className="h-6 w-6" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="text-lg font-bold">{descriptor.displayName}</div>
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <span>{descriptor.category} · 单独调用</span>
              <span
                className={cn(
                  "rounded px-1.5 py-0.5 text-[10px]",
                  inTauri ? "bg-green-500/10 text-green-600" : "bg-secondary text-muted-foreground"
                )}
              >
                {inTauri ? "本地执行" : "预览"}
              </span>
            </div>
          </div>
          <button onClick={onClose} className="rounded-md p-1 text-muted-foreground hover:bg-accent hover:text-foreground">
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* body */}
        <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[1fr_400px]">
          {/* left: input + params */}
          <div className="flex min-h-0 flex-col gap-5 overflow-y-auto border-r border-border p-5">
            {descriptor.inputs.length > 0 && (
              <section>
                <SectionTitle>输入</SectionTitle>
                <div className="mt-2 space-y-3">
                  {descriptor.inputs.map((port) => {
                    const val = inputs[port.name] ?? "";
                    return (
                      <div key={port.name}>
                        {descriptor.inputs.length > 1 && (
                          <div className="mb-1 text-[11px] text-muted-foreground">
                            {port.label} <span className="opacity-50">({port.type})</span>
                          </div>
                        )}
                        <div className="relative">
                          <textarea
                            rows={4}
                            value={val}
                            onChange={(e) => setInputs((p) => ({ ...p, [port.name]: e.target.value }))}
                            placeholder="输入文本或粘贴 Base64 / Hex 数据…"
                            className={cn(inputCls, "resize-y pb-6")}
                          />
                          <span className="pointer-events-none absolute bottom-2 right-3 text-[10px] text-muted-foreground">
                            {byteLen(val)} 字节
                          </span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </section>
            )}

            {descriptor.params.length > 0 && (
              <section>
                <SectionTitle>参数配置</SectionTitle>
                <div className="mt-2 space-y-3">
                  {rows.map((row, i) =>
                    row.kind === "single" ? (
                      <Field key={i} p={row.p} />
                    ) : (
                      <div
                        key={i}
                        className={cn(
                          "grid gap-2",
                          row.a.widget.kind === "text" || row.a.widget.kind === "number"
                            ? "grid-cols-[2fr_1fr]"
                            : "grid-cols-2"
                        )}
                      >
                        <Field p={row.a} />
                        <Field p={row.b} />
                      </div>
                    )
                  )}
                </div>
              </section>
            )}

            {descriptor.inputs.length === 0 && descriptor.params.length === 0 && (
              <div className="text-xs text-muted-foreground">该模块无需输入或参数，直接运行即可。</div>
            )}
          </div>

          {/* right: output preview */}
          <div className="flex min-h-0 flex-col p-5">
            <SectionTitle>输出预览</SectionTitle>
            <div className="mt-2 flex border-b border-border text-xs">
              {(
                [
                  ["result", "结果"],
                  ["logs", "日志"],
                  ["history", "历史"],
                ] as const
              ).map(([t, l]) => (
                <button
                  key={t}
                  onClick={() => setTab(t)}
                  className={cn(
                    "px-3 py-2 transition-colors",
                    tab === t
                      ? "border-b-2 border-primary font-medium text-primary"
                      : "text-muted-foreground hover:text-foreground"
                  )}
                >
                  {l}
                </button>
              ))}
            </div>

            <div className="min-h-0 flex-1 overflow-y-auto py-3">
              {tab === "result" &&
                (error ? (
                  <div className="whitespace-pre-wrap rounded-lg bg-destructive/10 p-3 text-xs text-destructive">
                    {error}
                  </div>
                ) : !outputs ? (
                  <EmptyBox text="运行后在这里查看结果" />
                ) : (
                  <div className="space-y-3">
                    {Object.entries(outputs).map(([k, v]) => (
                      <div key={k}>
                        <div className="mb-1 flex items-center justify-between">
                          <span className="text-[11px] font-medium text-muted-foreground">
                            {descriptor.outputs.find((o) => o.name === k)?.label ?? k}
                          </span>
                          {v.type !== "image" && <CopyBtn text={outText(v)} />}
                        </div>
                        {v.type === "image" ? (
                          <img src={v.value} alt="" className="max-h-56 rounded-lg border border-border bg-white" />
                        ) : (
                          <pre className="max-h-56 select-text overflow-auto whitespace-pre-wrap break-all rounded-lg bg-background p-2.5 font-mono text-[11px]">
                            {outText(v)}
                          </pre>
                        )}
                      </div>
                    ))}
                  </div>
                ))}

              {tab === "logs" &&
                (logs.length === 0 ? (
                  <EmptyBox text="暂无日志" />
                ) : (
                  <div className="space-y-1 font-mono text-[10px]">
                    {logs.map((l, i) => (
                      <div key={i} className="flex gap-2">
                        <span className="text-muted-foreground">{l.time}</span>
                        <span
                          className={cn(
                            l.level === "error"
                              ? "text-destructive"
                              : l.level === "success"
                                ? "text-green-600"
                                : "text-muted-foreground"
                          )}
                        >
                          {l.message}
                        </span>
                      </div>
                    ))}
                  </div>
                ))}

              {tab === "history" &&
                (history.length === 0 ? (
                  <EmptyBox text="暂无历史" />
                ) : (
                  <div className="space-y-1.5">
                    {history.map((h, i) => (
                      <button
                        key={i}
                        onClick={() => restore(h)}
                        className="flex w-full items-center gap-2 rounded-lg border border-border p-2 text-left text-[11px] transition-colors hover:border-primary hover:bg-accent"
                        title="点击回填参数与结果"
                      >
                        <span className={cn("h-2 w-2 shrink-0 rounded-full", h.error ? "bg-destructive" : "bg-green-500")} />
                        <span className="shrink-0 text-muted-foreground">{h.time}</span>
                        <span className="flex-1 truncate font-mono">
                          {h.error
                            ? "失败"
                            : h.outputs
                              ? outText(Object.values(h.outputs)[0] ?? { type: "none" }).slice(0, 40)
                              : ""}
                        </span>
                        <span className="shrink-0 text-muted-foreground">{h.elapsed}ms</span>
                      </button>
                    ))}
                  </div>
                ))}
            </div>

            {/* status panel */}
            <div className="mt-2 space-y-2 rounded-lg border border-border bg-secondary/30 p-3 text-xs">
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">状态</span>
                <span className="flex items-center gap-1 font-medium" style={{ color: status.c }}>
                  <Clock className="h-3 w-3" />
                  {status.t}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">耗时</span>
                <span className="font-mono">{elapsed != null ? `${elapsed} ms` : "--"}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">输出大小</span>
                <span className="font-mono">{outputs ? `${outputSize(outputs)} 字节` : "--"}</span>
              </div>
            </div>
          </div>
        </div>

        {/* footer */}
        <div className="flex items-center justify-between border-t border-border px-5 py-3">
          <button
            onClick={reset}
            className="flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
          >
            <RotateCcw className="h-3.5 w-3.5" />
            重置参数
          </button>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={onClose}>
              关闭
            </Button>
            <Button size="sm" onClick={run} disabled={running}>
              <Play className="mr-1 h-3.5 w-3.5" />
              {running ? "运行中…" : "运行"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
