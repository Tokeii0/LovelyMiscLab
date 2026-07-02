import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, FileCode2, FolderOpen, Loader2, Plus, Trash2, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { TOOLS } from "@/lib/tools";
import type {
  InputDelivery,
  NodeDescriptor,
  OutputDelivery,
  ParamSpec,
  ParamWidget,
  PortType,
  ScriptModule,
} from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useScriptDialogStore } from "@/store/scriptDialog";

const COLORS = ["#8b5cf6", "#f43f5e", "#06b6d4", "#22c55e", "#f59e0b", "#3b82f6"];
const PORT_TYPES: PortType[] = ["text", "bytes", "number", "bool", "json", "stringList", "any"];
const IN_DELIV: { v: InputDelivery; l: string }[] = [
  { v: "arg", l: "命令行参数" },
  { v: "stdin", l: "标准输入" },
  { v: "file", l: "临时文件" },
];
const OUT_DELIV: { v: OutputDelivery; l: string }[] = [
  { v: "stdoutJson", l: "stdout(JSON键)" },
  { v: "file", l: "临时文件" },
];
const WIDGET_KINDS: { v: ParamWidget["kind"]; l: string }[] = [
  { v: "text", l: "文本" },
  { v: "number", l: "数字" },
  { v: "slider", l: "滑块" },
  { v: "select", l: "下拉" },
  { v: "toggle", l: "开关" },
  { v: "file", l: "文件" },
];

interface InDraft { key: string; name: string; label: string; portType: PortType; delivery: InputDelivery }
interface OutDraft { key: string; name: string; label: string; portType: PortType; delivery: OutputDelivery }
interface ParamDraft {
  key: string; name: string; label: string; kind: ParamWidget["kind"];
  options: string; min: number; max: number; step: number; multiline: boolean; def: string;
}

let kc = 0;
const nk = () => `k${kc++}`;

const inputEl = "rounded border border-input bg-background px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-ring";
const labelEl = "mb-1 block text-xs font-medium text-muted-foreground";

function slugify(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "");
}

function toParamSpec(d: ParamDraft): ParamSpec {
  let widget: ParamWidget;
  let def: unknown;
  switch (d.kind) {
    case "number":
    case "slider":
      widget = { kind: d.kind, min: d.min, max: d.max, step: d.step };
      def = parseFloat(d.def) || 0;
      break;
    case "toggle":
      widget = { kind: "toggle" };
      def = ["true", "1", "yes", "on", "是"].includes(d.def.trim().toLowerCase());
      break;
    case "select": {
      const opts = d.options.split(",").map((s) => s.trim()).filter(Boolean);
      widget = { kind: "select", options: opts };
      def = d.def || opts[0] || "";
      break;
    }
    case "file":
      widget = { kind: "file" };
      def = d.def;
      break;
    default:
      widget = { kind: "text", multiline: d.multiline };
      def = d.def;
  }
  return { name: d.name, label: d.label || d.name, widget, default: def };
}

function scriptDescriptor(m: ScriptModule): NodeDescriptor {
  return {
    id: m.id,
    category: m.category || "自定义",
    displayName: m.name,
    description: m.description,
    color: m.color || "#8b5cf6",
    inputs: m.inputs.map((p) => ({ name: p.name, label: p.label, type: p.portType, required: false })),
    outputs: m.outputs.map((p) => ({ name: p.name, label: p.label, type: p.portType, required: false })),
    params: m.params,
    cost: "heavy",
  };
}

export function CreateScriptNodeDialog() {
  const open_ = useScriptDialogStore((s) => s.open);
  const setOpen = useScriptDialogStore((s) => s.setOpen);
  const setDescriptors = useDescriptorStore((s) => s.setDescriptors);

  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [color, setColor] = useState(COLORS[0]);
  const [commandSel, setCommandSel] = useState("__custom__");
  const [commandPath, setCommandPath] = useState("");
  const [argsTemplate, setArgsTemplate] = useState("");
  const [workingDir, setWorkingDir] = useState("");
  const [timeout, setTimeoutS] = useState(30);
  const [inputs, setInputs] = useState<InDraft[]>([]);
  const [params, setParams] = useState<ParamDraft[]>([]);
  const [outputs, setOutputs] = useState<OutDraft[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  if (!open_) return null;

  const reset = () => {
    setName(""); setDesc(""); setColor(COLORS[0]);
    setCommandSel("__custom__"); setCommandPath(""); setArgsTemplate("");
    setWorkingDir(""); setTimeoutS(30);
    setInputs([]); setParams([]); setOutputs([]); setError("");
  };
  const close = () => { if (!saving) { setOpen(false); reset(); } };

  const pick = async (set: (p: string) => void) => {
    if (!inTauri) return;
    const f = await open({ multiple: false, directory: false });
    if (typeof f === "string") set(f);
  };
  const pickDir = async () => {
    if (!inTauri) return;
    const d = await open({ directory: true });
    if (typeof d === "string") setWorkingDir(d);
  };
  const insertScriptPath = async () => {
    if (!inTauri) return;
    const f = await open({ multiple: false, directory: false });
    if (typeof f === "string") {
      const q = f.includes(" ") ? `"${f}"` : f;
      setArgsTemplate((t) => (t.trim() ? `${t} ${q}` : q));
    }
  };

  const upd = <T extends { key: string }>(
    set: React.Dispatch<React.SetStateAction<T[]>>,
    key: string,
    patch: Partial<T>
  ) => set((xs) => xs.map((x) => (x.key === key ? { ...x, ...patch } : x)));
  const del = <T extends { key: string }>(set: React.Dispatch<React.SetStateAction<T[]>>, key: string) =>
    set((xs) => xs.filter((x) => x.key !== key));

  const save = async () => {
    const command = commandSel === "__custom__" ? commandPath.trim() : commandSel;
    if (!name.trim()) return setError("请填写节点名称");
    if (!command) return setError("请选择工具或填写可执行程序路径");
    if (inputs.filter((i) => i.delivery === "stdin").length > 1)
      return setError("最多只能有一个「标准输入」端口");
    if (inputs.some((i) => !i.name.trim()) || outputs.some((o) => !o.name.trim()) || params.some((p) => !p.name.trim()))
      return setError("每个输入/参数/输出都需要一个名称");

    setSaving(true);
    setError("");
    try {
      const module: ScriptModule = {
        id: `script_${slugify(name) || "node"}_${Math.random().toString(36).slice(2, 7)}`,
        name: name.trim(),
        category: "自定义",
        color,
        description: desc.trim(),
        command,
        argsTemplate,
        workingDir: workingDir.trim() || null,
        timeoutSecs: Math.max(0, Math.floor(timeout)),
        inputs: inputs.map((i) => ({ name: i.name.trim(), label: i.label.trim() || i.name.trim(), portType: i.portType, delivery: i.delivery })),
        params: params.map(toParamSpec),
        outputs: outputs.map((o) => ({ name: o.name.trim(), label: o.label.trim() || o.name.trim(), portType: o.portType, delivery: o.delivery })),
      };
      if (inTauri) {
        await api.saveScriptModule(module);
        setDescriptors(await api.listNodeDescriptors());
      } else {
        const d = scriptDescriptor(module);
        setDescriptors([...useDescriptorStore.getState().list.filter((x) => x.id !== d.id), d]);
      }
      setOpen(false);
      reset();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[75] flex items-center justify-center bg-black/50 p-4" onClick={close}>
      <div
        className="flex max-h-[92vh] w-[720px] max-w-[97vw] flex-col rounded-xl border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border p-4">
          <span className="flex h-9 w-9 items-center justify-center rounded-lg" style={{ background: `${color}22`, color }}>
            <FileCode2 className="h-5 w-5" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="text-base font-semibold">脚本节点</div>
            <div className="text-xs text-muted-foreground">把你自己的脚本 / 程序接入为一个节点</div>
          </div>
          <button onClick={close} className="text-muted-foreground hover:text-foreground">
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="flex items-start gap-2 border-b border-border bg-amber-500/10 px-4 py-2 text-[11px] text-amber-700 dark:text-amber-400">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>该节点会在本机执行你配置的程序，请只添加你信任的脚本。</span>
        </div>

        <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
          {/* name + color */}
          <div className="flex items-end gap-3">
            <div className="flex-1">
              <label className={labelEl}>名称</label>
              <input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="例如：我的解密脚本" className={`w-full ${inputEl}`} />
            </div>
            <div>
              <label className={labelEl}>颜色</label>
              <div className="flex gap-1.5 py-1">
                {COLORS.map((c) => (
                  <button key={c} onClick={() => setColor(c)} className="h-5 w-5 rounded-full" style={{ background: c, boxShadow: color === c ? `0 0 0 2px ${c}` : "none" }} />
                ))}
              </div>
            </div>
          </div>

          <div>
            <label className={labelEl}>描述（可选）</label>
            <input value={desc} onChange={(e) => setDesc(e.target.value)} placeholder="这个节点做什么" className={`w-full ${inputEl}`} />
          </div>

          {/* command */}
          <div>
            <label className={labelEl}>可执行程序</label>
            <div className="flex gap-2">
              <select value={commandSel} onChange={(e) => setCommandSel(e.target.value)} className={inputEl}>
                <option value="__custom__">自定义路径…</option>
                {TOOLS.map((t) => (
                  <option key={t.key} value={`$tool:${t.key}`}>已配置：{t.label}</option>
                ))}
              </select>
              {commandSel === "__custom__" && (
                <>
                  <input value={commandPath} onChange={(e) => setCommandPath(e.target.value)} placeholder="python.exe 或 D:\\tools\\my.exe" className={`flex-1 ${inputEl}`} />
                  <button onClick={() => void pick(setCommandPath)} title="选择程序" className="rounded-md border border-border p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground">
                    <FolderOpen className="h-3.5 w-3.5" />
                  </button>
                </>
              )}
              {commandSel !== "__custom__" && (
                <span className="flex items-center text-[11px] text-muted-foreground">将使用「设置」中配置的路径</span>
              )}
            </div>
          </div>

          {/* args template */}
          <div>
            <label className={labelEl}>参数模板</label>
            <div className="flex gap-2">
              <input value={argsTemplate} onChange={(e) => setArgsTemplate(e.target.value)} placeholder={`"C:\\my scripts\\s.py" --mode {mode} {data}`} className={`flex-1 ${inputEl} font-mono`} />
              <Button variant="outline" size="sm" onClick={() => void insertScriptPath()}>浏览脚本…</Button>
            </div>
            <p className="mt-1 text-[10px] text-muted-foreground">用 {`{端口名}`} / {`{参数名}`} 引用；含空格的值会自动作为单个参数，无需手动加引号。</p>
          </div>

          <div className="flex gap-3">
            <div className="flex-1">
              <label className={labelEl}>工作目录（可选）</label>
              <div className="flex gap-2">
                <input value={workingDir} onChange={(e) => setWorkingDir(e.target.value)} placeholder="默认临时目录" className={`flex-1 ${inputEl}`} />
                <button onClick={() => void pickDir()} title="选择目录" className="rounded-md border border-border p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground">
                  <FolderOpen className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
            <div className="w-28">
              <label className={labelEl}>超时(秒)</label>
              <input type="number" min={0} value={timeout} onChange={(e) => setTimeoutS(Number(e.target.value))} className={`w-full ${inputEl}`} />
            </div>
          </div>

          {/* inputs */}
          <Section
            title="输入端口"
            onAdd={() => setInputs((x) => [...x, { key: nk(), name: "", label: "", portType: "text", delivery: "arg" }])}
            empty="无输入（源节点）"
            items={inputs}
          >
            {inputs.map((p) => (
              <div key={p.key} className="flex items-center gap-1.5">
                <input value={p.name} onChange={(e) => upd(setInputs, p.key, { name: e.target.value })} placeholder="name" className={`w-24 ${inputEl}`} />
                <input value={p.label} onChange={(e) => upd(setInputs, p.key, { label: e.target.value })} placeholder="标签" className={`flex-1 ${inputEl}`} />
                <select value={p.portType} onChange={(e) => upd(setInputs, p.key, { portType: e.target.value as PortType })} className={inputEl}>
                  {PORT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                </select>
                <select value={p.delivery} onChange={(e) => upd(setInputs, p.key, { delivery: e.target.value as InputDelivery })} className={inputEl}>
                  {IN_DELIV.map((d) => <option key={d.v} value={d.v}>{d.l}</option>)}
                </select>
                <RowDel onClick={() => del(setInputs, p.key)} />
              </div>
            ))}
          </Section>

          {/* params */}
          <Section
            title="参数"
            onAdd={() => setParams((x) => [...x, { key: nk(), name: "", label: "", kind: "text", options: "", min: 0, max: 100, step: 1, multiline: false, def: "" }])}
            empty="无参数"
            items={params}
          >
            {params.map((p) => (
              <div key={p.key} className="space-y-1 rounded-md border border-border/60 p-1.5">
                <div className="flex items-center gap-1.5">
                  <input value={p.name} onChange={(e) => upd(setParams, p.key, { name: e.target.value })} placeholder="name" className={`w-24 ${inputEl}`} />
                  <input value={p.label} onChange={(e) => upd(setParams, p.key, { label: e.target.value })} placeholder="标签" className={`flex-1 ${inputEl}`} />
                  <select value={p.kind} onChange={(e) => upd(setParams, p.key, { kind: e.target.value as ParamWidget["kind"] })} className={inputEl}>
                    {WIDGET_KINDS.map((w) => <option key={w.v} value={w.v}>{w.l}</option>)}
                  </select>
                  <RowDel onClick={() => del(setParams, p.key)} />
                </div>
                <div className="flex items-center gap-1.5 pl-1">
                  {p.kind === "select" && (
                    <input value={p.options} onChange={(e) => upd(setParams, p.key, { options: e.target.value })} placeholder="选项(逗号分隔)" className={`flex-1 ${inputEl}`} />
                  )}
                  {(p.kind === "number" || p.kind === "slider") && (
                    <>
                      <input type="number" value={p.min} onChange={(e) => upd(setParams, p.key, { min: Number(e.target.value) })} title="最小" className={`w-16 ${inputEl}`} />
                      <input type="number" value={p.max} onChange={(e) => upd(setParams, p.key, { max: Number(e.target.value) })} title="最大" className={`w-16 ${inputEl}`} />
                      <input type="number" value={p.step} onChange={(e) => upd(setParams, p.key, { step: Number(e.target.value) })} title="步长" className={`w-16 ${inputEl}`} />
                    </>
                  )}
                  {p.kind === "text" && (
                    <label className="flex items-center gap-1 text-[11px] text-muted-foreground">
                      <input type="checkbox" checked={p.multiline} onChange={(e) => upd(setParams, p.key, { multiline: e.target.checked })} className="h-3.5 w-3.5 accent-primary" /> 多行
                    </label>
                  )}
                  <input value={p.def} onChange={(e) => upd(setParams, p.key, { def: e.target.value })} placeholder="默认值" className={`flex-1 ${inputEl}`} />
                </div>
              </div>
            ))}
          </Section>

          {/* outputs */}
          <Section
            title="输出端口"
            onAdd={() => setOutputs((x) => [...x, { key: nk(), name: "", label: "", portType: "text", delivery: "stdoutJson" }])}
            empty="无输出"
            items={outputs}
          >
            {outputs.map((p) => (
              <div key={p.key} className="flex items-center gap-1.5">
                <input value={p.name} onChange={(e) => upd(setOutputs, p.key, { name: e.target.value })} placeholder="name" className={`w-24 ${inputEl}`} />
                <input value={p.label} onChange={(e) => upd(setOutputs, p.key, { label: e.target.value })} placeholder="标签" className={`flex-1 ${inputEl}`} />
                <select value={p.portType} onChange={(e) => upd(setOutputs, p.key, { portType: e.target.value as PortType })} className={inputEl}>
                  {PORT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                </select>
                <select value={p.delivery} onChange={(e) => upd(setOutputs, p.key, { delivery: e.target.value as OutputDelivery })} className={inputEl}>
                  {OUT_DELIV.map((d) => <option key={d.v} value={d.v}>{d.l}</option>)}
                </select>
                <RowDel onClick={() => del(setOutputs, p.key)} />
              </div>
            ))}
          </Section>

          {error && <div className="whitespace-pre-wrap rounded-lg bg-destructive/10 p-2.5 text-xs text-destructive">{error}</div>}
          {!inTauri && (
            <div className="text-[11px] text-muted-foreground">提示：浏览器预览仅将节点加入当前会话演示；持久保存与运行需在桌面应用中进行。</div>
          )}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-border p-4">
          <Button variant="outline" size="sm" onClick={close} disabled={saving}>取消</Button>
          <Button size="sm" onClick={save} disabled={saving}>
            {saving ? <><Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> 保存中…</> : <><FileCode2 className="mr-1 h-3.5 w-3.5" /> 保存节点</>}
          </Button>
        </div>
      </div>
    </div>
  );
}

function Section({ title, onAdd, empty, items, children }: {
  title: string; onAdd: () => void; empty: string; items: unknown[]; children: React.ReactNode;
}) {
  return (
    <div>
      <div className="mb-1 flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">{title}</span>
        <button onClick={onAdd} className="flex items-center gap-1 rounded-md border border-border px-1.5 py-0.5 text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground">
          <Plus className="h-3 w-3" /> 添加
        </button>
      </div>
      {items.length === 0 ? (
        <div className="rounded-md bg-muted/40 px-2 py-1.5 text-[11px] text-muted-foreground">{empty}</div>
      ) : (
        <div className="space-y-1.5">{children}</div>
      )}
    </div>
  );
}

function RowDel({ onClick }: { onClick: () => void }) {
  return (
    <button onClick={onClick} title="删除" className="shrink-0 rounded-md p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive">
      <Trash2 className="h-3.5 w-3.5" />
    </button>
  );
}
