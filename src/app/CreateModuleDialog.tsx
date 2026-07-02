import { useEffect, useState } from "react";
import { Boxes, Loader2, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { CompositeModule } from "@/lib/types";
import {
  buildEncapsulation,
  compositeDescriptor,
  type DetectedPort,
  type Encapsulation,
  newModuleId,
  toBoundaryPorts,
} from "@/flow/encapsulate";
import { useDescriptorStore } from "@/store/descriptors";
import { useModuleDialogStore } from "@/store/moduleDialog";

const COLORS = ["#8b5cf6", "#f43f5e", "#06b6d4", "#22c55e", "#f59e0b", "#3b82f6"];

function PortRows({
  ports,
  onToggle,
  onLabel,
  empty,
}: {
  ports: DetectedPort[];
  onToggle: (key: string) => void;
  onLabel: (key: string, label: string) => void;
  empty: string;
}) {
  if (ports.length === 0) {
    return <div className="rounded-md bg-muted/40 px-2 py-1.5 text-[11px] text-muted-foreground">{empty}</div>;
  }
  return (
    <div className="space-y-1">
      {ports.map((p) => (
        <div key={p.key} className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={p.include}
            onChange={() => onToggle(p.key)}
            className="h-3.5 w-3.5 accent-primary"
          />
          <input
            value={p.label}
            disabled={!p.include}
            onChange={(e) => onLabel(p.key, e.target.value)}
            className="min-w-0 flex-1 rounded border border-input bg-background px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-ring disabled:opacity-50"
          />
          <span className="shrink-0 rounded bg-secondary px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
            {p.portType}
          </span>
        </div>
      ))}
    </div>
  );
}

export function CreateModuleDialog() {
  const open = useModuleDialogStore((s) => s.open);
  const setOpen = useModuleDialogStore((s) => s.setOpen);
  const setDescriptors = useDescriptorStore((s) => s.setDescriptors);

  const [enc, setEnc] = useState<Encapsulation | null>(null);
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [color, setColor] = useState(COLORS[0]);
  const [inputs, setInputs] = useState<DetectedPort[]>([]);
  const [outputs, setOutputs] = useState<DetectedPort[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!open) return;
    const e = buildEncapsulation();
    setEnc(e);
    setInputs(e?.inputs ?? []);
    setOutputs(e?.outputs ?? []);
    setName("");
    setDesc("");
    setColor(COLORS[0]);
    setError("");
  }, [open]);

  if (!open) return null;

  const close = () => {
    if (!saving) setOpen(false);
  };
  const toggle = (list: DetectedPort[], set: (v: DetectedPort[]) => void, key: string) =>
    set(list.map((p) => (p.key === key ? { ...p, include: !p.include } : p)));
  const relabel = (list: DetectedPort[], set: (v: DetectedPort[]) => void, key: string, label: string) =>
    set(list.map((p) => (p.key === key ? { ...p, label } : p)));

  const save = async () => {
    if (!enc || !name.trim() || saving) return;
    setSaving(true);
    setError("");
    try {
      const module: CompositeModule = {
        id: newModuleId(name),
        name: name.trim(),
        category: "自定义",
        color,
        description: desc.trim(),
        graph: enc.graph,
        inputs: toBoundaryPorts(inputs),
        outputs: toBoundaryPorts(outputs),
      };
      if (inTauri) {
        await api.saveCompositeModule(module);
        setDescriptors(await api.listNodeDescriptors());
      } else {
        const d = compositeDescriptor(module);
        const cur = useDescriptorStore.getState().list.filter((x) => x.id !== d.id);
        setDescriptors([...cur, d]);
      }
      setOpen(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const nodeCount = enc?.graph.nodes.length ?? 0;

  return (
    <div className="fixed inset-0 z-[75] flex items-center justify-center bg-black/50 p-4" onClick={close}>
      <div
        className="flex max-h-[90vh] w-[560px] max-w-[95vw] flex-col rounded-xl border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border p-4">
          <span
            className="flex h-9 w-9 items-center justify-center rounded-lg"
            style={{ background: `${color}22`, color }}
          >
            <Boxes className="h-5 w-5" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="text-base font-semibold">封装为模块</div>
            <div className="text-xs text-muted-foreground">
              把选中的 {nodeCount} 个节点打包成一个可复用模块
            </div>
          </div>
          <button onClick={close} className="text-muted-foreground hover:text-foreground">
            <X className="h-5 w-5" />
          </button>
        </div>

        {!enc ? (
          <div className="p-6 text-center text-sm text-muted-foreground">
            请先在画布上框选（或按住 Shift 点选）至少一个节点，再封装为模块。
          </div>
        ) : (
          <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
            <div>
              <label className="mb-1 block text-xs font-medium text-muted-foreground">名称</label>
              <input
                autoFocus
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="例如：套娃 Base64 解码"
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
                onKeyDown={(e) => {
                  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") void save();
                }}
              />
            </div>

            <div className="flex items-center gap-3">
              <span className="text-xs font-medium text-muted-foreground">颜色</span>
              <div className="flex gap-1.5">
                {COLORS.map((c) => (
                  <button
                    key={c}
                    onClick={() => setColor(c)}
                    className="h-5 w-5 rounded-full ring-offset-2 ring-offset-card transition"
                    style={{ background: c, boxShadow: color === c ? `0 0 0 2px ${c}` : "none" }}
                    title={c}
                  />
                ))}
              </div>
            </div>

            <div>
              <label className="mb-1 block text-xs font-medium text-muted-foreground">描述（可选）</label>
              <input
                value={desc}
                onChange={(e) => setDesc(e.target.value)}
                placeholder="这个模块做什么"
                className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <div className="mb-1 text-xs font-medium text-muted-foreground">对外输入</div>
                <PortRows
                  ports={inputs}
                  onToggle={(k) => toggle(inputs, setInputs, k)}
                  onLabel={(k, l) => relabel(inputs, setInputs, k, l)}
                  empty="无（子图内部已全部连接）"
                />
              </div>
              <div>
                <div className="mb-1 text-xs font-medium text-muted-foreground">对外输出</div>
                <PortRows
                  ports={outputs}
                  onToggle={(k) => toggle(outputs, setOutputs, k)}
                  onLabel={(k, l) => relabel(outputs, setOutputs, k, l)}
                  empty="无可用输出"
                />
              </div>
            </div>

            {error && (
              <div className="whitespace-pre-wrap rounded-lg bg-destructive/10 p-2.5 text-xs text-destructive">
                {error}
              </div>
            )}
            {!inTauri && (
              <div className="text-[11px] text-muted-foreground">
                提示：浏览器预览仅将模块加入当前会话演示；持久保存与运行需在桌面应用中进行。
              </div>
            )}
          </div>
        )}

        <div className="flex items-center justify-end gap-2 border-t border-border p-4">
          <Button variant="outline" size="sm" onClick={() => setOpen(false)} disabled={saving}>
            取消
          </Button>
          <Button size="sm" onClick={save} disabled={saving || !enc || !name.trim()}>
            {saving ? (
              <>
                <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> 保存中…
              </>
            ) : (
              <>
                <Boxes className="mr-1 h-3.5 w-3.5" /> 保存模块
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}
