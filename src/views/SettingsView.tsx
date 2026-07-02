import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Bot,
  Check,
  Eye,
  FolderOpen,
  Loader2,
  Save,
  Server,
  Wrench,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { api, type AppSettings, type ModelConfig, type ToolStatus } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { TOOLS } from "@/lib/tools";
import { McpPanel } from "@/views/McpPanel";

const EMPTY: AppSettings = {
  ai: {
    llm: { model: "", apiKey: "", baseUrl: "" },
    vision: { model: "", apiKey: "", baseUrl: "" },
  },
  outputDir: "",
  tools: {},
};

function Field({
  label,
  value,
  onChange,
  placeholder,
  password,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  password?: boolean;
}) {
  return (
    <label className="block">
      <span className="text-[11px] text-muted-foreground">{label}</span>
      <input
        type={password ? "password" : "text"}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className="mt-1 w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
      />
    </label>
  );
}

function ModelCard({
  title,
  icon: Icon,
  cfg,
  onChange,
}: {
  title: string;
  icon: typeof Bot;
  cfg: ModelConfig;
  onChange: (field: keyof ModelConfig, v: string) => void;
}) {
  return (
    <div className="flex-1 space-y-3 rounded-xl border border-border bg-card p-4">
      <div className="flex items-center gap-2 text-sm font-medium">
        <span className="flex h-6 w-6 items-center justify-center rounded-md bg-primary/10 text-primary">
          <Icon className="h-3.5 w-3.5" />
        </span>
        {title}
      </div>
      <Field
        label="Base URL"
        value={cfg.baseUrl}
        onChange={(v) => onChange("baseUrl", v)}
        placeholder="https://api.openai.com/v1"
      />
      <Field
        label="模型名称"
        value={cfg.model}
        onChange={(v) => onChange("model", v)}
        placeholder="gpt-4o-mini"
      />
      <Field
        label="API Key"
        value={cfg.apiKey}
        onChange={(v) => onChange("apiKey", v)}
        placeholder="sk-…"
        password
      />
    </div>
  );
}

function ToolStatusBadge({ status }: { status: ToolStatus | "checking" | undefined }) {
  if (status === "checking")
    return (
      <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
        <Loader2 className="h-3 w-3 animate-spin" /> 检测中
      </span>
    );
  if (!status) return <span className="text-[11px] text-muted-foreground/50">未检测</span>;
  if (status.available)
    return (
      <span className="flex items-center gap-1 text-[11px] text-green-600" title={status.version}>
        <Check className="h-3 w-3" />
        <span className="max-w-[160px] truncate">{status.version || "可用"}</span>
      </span>
    );
  return (
    <span className="flex items-center gap-1 text-[11px] text-destructive">
      <X className="h-3 w-3" /> 未找到
    </span>
  );
}

const TABS = [
  { id: "ai" as const, label: "AI 模型", icon: Bot },
  { id: "output" as const, label: "输出目录", icon: FolderOpen },
  { id: "tools" as const, label: "外部工具", icon: Wrench },
  { id: "mcp" as const, label: "MCP 服务", icon: Server },
];
type Tab = (typeof TABS)[number]["id"];

export function SettingsView() {
  const [s, setS] = useState<AppSettings>(EMPTY);
  const [status, setStatus] = useState<Record<string, ToolStatus | "checking">>({});
  const [saved, setSaved] = useState(false);
  const [tab, setTab] = useState<Tab>("ai");

  useEffect(() => {
    if (inTauri) api.getSettings().then(setS).catch(() => {});
  }, []);

  const setAi = (g: "llm" | "vision", field: keyof ModelConfig, v: string) =>
    setS((c) => ({ ...c, ai: { ...c.ai, [g]: { ...c.ai[g], [field]: v } } }));
  const setTool = (k: string, v: string) =>
    setS((c) => ({ ...c, tools: { ...c.tools, [k]: v } }));

  const save = async () => {
    if (inTauri) {
      try {
        await api.setSettings(s);
      } catch (e) {
        console.error("setSettings failed", e);
      }
    }
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  const pickDir = async () => {
    if (!inTauri) return;
    const d = await open({ directory: true });
    if (typeof d === "string") setS((c) => ({ ...c, outputDir: d }));
  };
  const pickTool = async (k: string) => {
    if (!inTauri) return;
    const f = await open({ multiple: false, directory: false });
    if (typeof f === "string") setTool(k, f);
  };
  const detect = async (k: string, arg: string) => {
    if (!inTauri) return;
    setStatus((p) => ({ ...p, [k]: "checking" }));
    try {
      const r = await api.detectTool(s.tools[k] ?? "", arg);
      setStatus((p) => ({ ...p, [k]: r }));
    } catch {
      setStatus((p) => ({ ...p, [k]: { available: false, version: "" } }));
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border px-6 py-4">
        <div>
          <h1 className="text-lg font-semibold">设置</h1>
          <p className="text-xs text-muted-foreground">配置 AI 模型、输出目录、外部工具与 MCP 服务。</p>
        </div>
        {tab !== "mcp" && (
          <Button size="sm" onClick={save}>
            {saved ? (
              <>
                <Check className="mr-1 h-3.5 w-3.5" /> 已保存
              </>
            ) : (
              <>
                <Save className="mr-1 h-3.5 w-3.5" /> 保存
              </>
            )}
          </Button>
        )}
      </div>

      <div className="flex min-h-0 flex-1">
        {/* category sub-nav */}
        <nav className="w-44 shrink-0 space-y-1 border-r border-border p-3">
          {TABS.map((t) => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={`flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-xs transition-colors ${
                tab === t.id
                  ? "bg-primary/10 font-medium text-primary"
                  : "text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              <t.icon className="h-4 w-4" />
              {t.label}
            </button>
          ))}
        </nav>

        {/* active category */}
        <div className="flex-1 space-y-6 overflow-y-auto p-6">
          {tab === "ai" && (
            <section>
              <h2 className="mb-2 text-sm font-semibold">AI 模型</h2>
              <div className="flex flex-col gap-3 lg:flex-row">
                <ModelCard
                  title="文本模型 (LLM)"
                  icon={Bot}
                  cfg={s.ai.llm}
                  onChange={(f, v) => setAi("llm", f, v)}
                />
                <ModelCard
                  title="识图模型 (Vision)"
                  icon={Eye}
                  cfg={s.ai.vision}
                  onChange={(f, v) => setAi("vision", f, v)}
                />
              </div>
            </section>
          )}

          {tab === "output" && (
            <section>
              <h2 className="mb-2 text-sm font-semibold">输出目录</h2>
              <div className="rounded-xl border border-border bg-card p-4">
                <p className="mb-2 text-[11px] text-muted-foreground">
                  「文件输出」节点会将结果写入此目录。
                </p>
                <div className="flex items-center gap-2">
                  <input
                    value={s.outputDir}
                    onChange={(e) => setS((c) => ({ ...c, outputDir: e.target.value }))}
                    placeholder="例如 D:\\ctf\\output"
                    className="flex-1 rounded-md border border-input bg-background px-2.5 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                  <Button variant="outline" size="sm" onClick={pickDir}>
                    <FolderOpen className="mr-1 h-3.5 w-3.5" /> 选择
                  </Button>
                </div>
              </div>
            </section>
          )}

          {tab === "tools" && (
            <section>
              <div className="mb-2 flex items-center gap-2">
                <Wrench className="h-4 w-4 text-muted-foreground" />
                <h2 className="text-sm font-semibold">外部工具</h2>
              </div>
              <div className="divide-y divide-border overflow-hidden rounded-xl border border-border bg-card">
                {TOOLS.map((t) => (
                  <div key={t.key} className="flex items-center gap-3 p-3">
                    <div className="w-20 shrink-0">
                      <div className="text-xs font-medium">{t.label}</div>
                      <div className="text-[10px] text-muted-foreground">{t.hint}</div>
                    </div>
                    <input
                      value={s.tools[t.key] ?? ""}
                      onChange={(e) => setTool(t.key, e.target.value)}
                      placeholder={`${t.label} 可执行文件路径`}
                      className="flex-1 rounded-md border border-input bg-background px-2.5 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
                    />
                    <div className="flex w-40 shrink-0 justify-end">
                      <ToolStatusBadge status={status[t.key]} />
                    </div>
                    <button
                      onClick={() => void pickTool(t.key)}
                      className="rounded-md border border-border p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
                      title="选择文件"
                    >
                      <FolderOpen className="h-3.5 w-3.5" />
                    </button>
                    <Button variant="outline" size="sm" onClick={() => void detect(t.key, t.arg)}>
                      检测
                    </Button>
                    <button
                      onClick={() => {
                        setTool(t.key, "");
                        setStatus((p) => {
                          const n = { ...p };
                          delete n[t.key];
                          return n;
                        });
                      }}
                      className="rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                      title="清除"
                    >
                      <X className="h-3.5 w-3.5" />
                    </button>
                  </div>
                ))}
              </div>
            </section>
          )}

          {tab === "mcp" && <McpPanel />}

          {tab !== "mcp" && !inTauri && (
            <p className="text-[11px] text-muted-foreground">
              浏览器预览下设置不会保存，请在应用内配置。
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
