import { useState } from "react";
import { X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { NodeDescriptor, PortValue } from "@/lib/types";
import { nodeIcon } from "@/flow/nodeIcons";
import { WidgetRenderer } from "@/flow/WidgetRenderer";

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

export function ModuleRunDialog({
  descriptor,
  onClose,
}: {
  descriptor: NodeDescriptor;
  onClose: () => void;
}) {
  const [inputs, setInputs] = useState<Record<string, string>>({});
  const [params, setParams] = useState<Record<string, unknown>>(() => {
    const p: Record<string, unknown> = {};
    for (const s of descriptor.params) p[s.name] = s.default;
    return p;
  });
  const [outputs, setOutputs] = useState<Record<string, PortValue> | null>(null);
  const [error, setError] = useState("");
  const [running, setRunning] = useState(false);
  const Icon = nodeIcon(descriptor.id, descriptor.category);

  const run = async () => {
    if (!inTauri) {
      setError("浏览器预览无法执行模块，请在应用中运行。");
      return;
    }
    setRunning(true);
    setError("");
    setOutputs(null);
    const inputMap: Record<string, PortValue> = {};
    for (const port of descriptor.inputs) {
      inputMap[port.name] = { type: "text", value: inputs[port.name] ?? "" };
    }
    try {
      setOutputs(await api.runNode(descriptor.id, inputMap, params));
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[70] flex items-center justify-center bg-black/50"
      onClick={onClose}
    >
      <div
        className="flex max-h-[82vh] w-[520px] flex-col rounded-lg border border-border bg-card shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border p-3">
          <span
            className="flex h-7 w-7 items-center justify-center rounded-md"
            style={{ background: `${descriptor.color}18`, color: descriptor.color }}
          >
            <Icon className="h-4 w-4" />
          </span>
          <div className="flex-1">
            <div className="text-sm font-semibold">{descriptor.displayName}</div>
            <div className="text-[10px] text-muted-foreground">
              {descriptor.category} · 单独调用
            </div>
          </div>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 space-y-3 overflow-y-auto p-3 text-xs">
          {descriptor.inputs.length > 0 && (
            <div>
              <div className="mb-1 font-semibold text-muted-foreground">输入</div>
              {descriptor.inputs.map((port) => (
                <label key={port.name} className="mb-2 block">
                  <span className="text-[11px] text-muted-foreground">
                    {port.label} <span className="opacity-50">({port.type})</span>
                  </span>
                  <textarea
                    rows={2}
                    value={inputs[port.name] ?? ""}
                    onChange={(e) =>
                      setInputs((p) => ({ ...p, [port.name]: e.target.value }))
                    }
                    className="mt-0.5 w-full resize-none rounded border border-input bg-background px-2 py-1 text-xs"
                  />
                </label>
              ))}
            </div>
          )}

          {descriptor.params.length > 0 && (
            <div>
              <div className="mb-1 font-semibold text-muted-foreground">参数</div>
              {descriptor.params.map((s) => (
                <label key={s.name} className="mb-2 block">
                  <span className="text-[11px] text-muted-foreground">{s.label}</span>
                  <WidgetRenderer
                    spec={s}
                    value={params[s.name]}
                    onChange={(v) => setParams((p) => ({ ...p, [s.name]: v }))}
                  />
                </label>
              ))}
            </div>
          )}

          {error && (
            <div className="rounded bg-destructive/10 p-2 text-destructive">{error}</div>
          )}

          {outputs && (
            <div>
              <div className="mb-1 font-semibold text-muted-foreground">输出</div>
              {Object.entries(outputs).map(([k, v]) => (
                <div key={k} className="mb-2">
                  <div className="text-[10px] text-muted-foreground">
                    {descriptor.outputs.find((o) => o.name === k)?.label ?? k}
                  </div>
                  {v.type === "image" ? (
                    <img
                      src={v.value}
                      alt=""
                      className="max-h-48 rounded border border-border bg-white"
                    />
                  ) : (
                    <pre className="max-h-40 select-text overflow-auto whitespace-pre-wrap break-all rounded bg-background p-2 font-mono text-[10px]">
                      {outText(v)}
                    </pre>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="flex justify-end gap-2 border-t border-border p-3">
          <Button variant="outline" size="sm" onClick={onClose}>
            关闭
          </Button>
          <Button size="sm" onClick={run} disabled={running}>
            {running ? "运行中…" : "运行"}
          </Button>
        </div>
      </div>
    </div>
  );
}
