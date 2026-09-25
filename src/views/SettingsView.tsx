import { useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Bot,
  Check,
  Download,
  Eye,
  FolderOpen,
  Loader2,
  RefreshCw,
  Save,
  Server,
  SlidersHorizontal,
  Sparkles,
  Wrench,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { api, type AppSettings, type ModelConfig, type ToolStatus } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { TOOLS } from "@/lib/tools";
import { usePrefs } from "@/store/prefs";
import { useThemeStore } from "@/store/theme";
import { useUpdate } from "@/store/update";
import { McpPanel } from "@/views/McpPanel";
import { choose } from "@/store/confirm";
import { toast } from "@/store/toast";
import { useViewStore, VIEW_LABEL } from "@/store/view";
import { isAnyModalOpen } from "@/store/modal";

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

function UpdatePanel() {
  const [ver, setVer] = useState("");
  const status = useUpdate((s) => s.status);
  const info = useUpdate((s) => s.info);
  const error = useUpdate((s) => s.error);
  const check = useUpdate((s) => s.check);

  useEffect(() => {
    if (inTauri) api.appInfo().then((a) => setVer(a.version)).catch(() => {});
  }, []);

  const current = info?.current || ver;
  return (
    <section>
      <div className="mb-2 flex items-center gap-2">
        <Download className="h-4 w-4 text-muted-foreground" />
        <h2 className="text-sm font-semibold">软件更新</h2>
      </div>
      <div className="space-y-3 rounded-xl border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-xs text-muted-foreground">当前版本</div>
            <div className="text-sm font-medium">v{current || "?"}</div>
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => void check()}
            disabled={status === "checking" || !inTauri}
          >
            {status === "checking" ? (
              <>
                <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> 检查中
              </>
            ) : (
              <>
                <RefreshCw className="mr-1 h-3.5 w-3.5" /> 检查更新
              </>
            )}
          </Button>
        </div>
        {status === "uptodate" && (
          <div className="flex items-center gap-1.5 text-xs text-green-600">
            <Check className="h-3.5 w-3.5" /> 已是最新版本
          </div>
        )}
        {status === "available" && info && (
          <div className="flex items-center gap-1.5 text-xs text-primary">
            <Sparkles className="h-3.5 w-3.5" /> 发现新版本 v{info.latest} —— 已弹出更新窗口
          </div>
        )}
        {status === "error" && <div className="text-xs text-destructive">{error}</div>}
        <p className="text-[11px] text-muted-foreground">
          从 GitHub 最新 Release 拉取。启动时自动检查一次；确认后自动下载、替换并重启。
        </p>
        {!inTauri && (
          <p className="text-[11px] text-muted-foreground">浏览器预览下不可用，请在应用内使用。</p>
        )}
      </div>
    </section>
  );
}

function Choice<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: [T, string][];
  onChange: (v: T) => void;
}) {
  return (
    <div className="inline-flex rounded-md border border-border bg-background p-0.5">
      {options.map(([v, label]) => (
        <button
          key={v}
          onClick={() => onChange(v)}
          className={`rounded px-2.5 py-1 text-xs transition-colors ${
            value === v ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-accent"
          }`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

/** Interface preferences. They live in this browser profile and apply at once
 * (no 保存 needed), unlike the backend settings on the other tabs. */
function GeneralSettings() {
  const themePref = useThemeStore((s) => s.pref);
  const setThemePref = useThemeStore((s) => s.setPref);
  const edgeLabels = usePrefs((s) => s.edgeLabels);
  const galgame = usePrefs((s) => s.experimentalGalgame);
  const setPref = usePrefs((s) => s.set);
  const row = "flex items-center justify-between gap-4 border-b border-border py-3 last:border-0";
  return (
    <section>
      <h2 className="mb-1 text-sm font-semibold">界面</h2>
      <p className="mb-2 text-[11px] text-muted-foreground">修改后立即生效。</p>
      <div className="rounded-xl border border-border bg-card px-4">
        <div className={row}>
          <div>
            <div className="text-xs font-medium">主题</div>
            <div className="text-[11px] text-muted-foreground">「跟随系统」随操作系统的浅色/深色切换</div>
          </div>
          <Choice
            value={themePref}
            onChange={setThemePref}
            options={[
              ["system", "跟随系统"],
              ["light", "浅色"],
              ["dark", "深色"],
            ]}
          />
        </div>
        <div className={row}>
          <div>
            <div className="text-xs font-medium">连线数据标签</div>
            <div className="text-[11px] text-muted-foreground">在连线上显示数据类型与值预览</div>
          </div>
          <Choice
            value={edgeLabels}
            onChange={(v) => setPref("edgeLabels", v)}
            options={[
              ["focus", "悬停/选中时"],
              ["always", "始终"],
            ]}
          />
        </div>
      </div>

      <h2 className="mb-1 mt-6 text-sm font-semibold">实验功能</h2>
      <p className="mb-2 text-[11px] text-muted-foreground">仍在打磨中的功能，默认关闭。</p>
      <div className="rounded-xl border border-border bg-card px-4">
        <label className={`${row} cursor-pointer`}>
          <div>
            <div className="text-xs font-medium">故事模式（galgame）</div>
            <div className="text-[11px] text-muted-foreground">
              由 AI 角色引导逐步解题；需要先配置 AI 文本模型。开启后出现在左侧栏。
            </div>
          </div>
          <input
            type="checkbox"
            className="h-4 w-4"
            checked={galgame}
            onChange={(e) => setPref("experimentalGalgame", e.target.checked)}
          />
        </label>
      </div>
    </section>
  );
}

const TABS = [
  { id: "general" as const, label: "通用", icon: SlidersHorizontal },
  { id: "ai" as const, label: "AI 模型", icon: Bot },
  { id: "output" as const, label: "输出目录", icon: FolderOpen },
  { id: "tools" as const, label: "外部工具", icon: Wrench },
  { id: "mcp" as const, label: "MCP 服务", icon: Server },
  { id: "update" as const, label: "软件更新", icon: Download },
];
type Tab = (typeof TABS)[number]["id"];

export function SettingsView() {
  const [s, setS] = useState<AppSettings>(EMPTY);
  // Last persisted copy — the form is dirty while it differs from this.
  const [stored, setStored] = useState<AppSettings>(EMPTY);
  const [status, setStatus] = useState<Record<string, ToolStatus | "checking">>({});
  const [saved, setSaved] = useState(false);
  const [tab, setTab] = useState<Tab>("general");
  const dirty = useMemo(() => JSON.stringify(s) !== JSON.stringify(stored), [s, stored]);

  useEffect(() => {
    if (!inTauri) return;
    api
      .getSettings()
      .then((v) => {
        setS(v);
        setStored(v);
      })
      .catch((e) => toast.error("读取设置失败", { error: e }));
  }, []);

  const setAi = (g: "llm" | "vision", field: keyof ModelConfig, v: string) =>
    setS((c) => ({ ...c, ai: { ...c.ai, [g]: { ...c.ai[g], [field]: v } } }));
  const setTool = (k: string, v: string) =>
    setS((c) => ({ ...c, tools: { ...c.tools, [k]: v } }));

  const save = async (): Promise<boolean> => {
    if (!inTauri) {
      toast.info("浏览器预览中不会保存设置");
      return true;
    }
    try {
      await api.setSettings(s);
    } catch (e) {
      toast.error("保存设置失败", { error: e });
      return false;
    }
    setStored(s);
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
    return true;
  };

  // Leaving the page with unsaved edits asks first instead of silently dropping them.
  const saveRef = useRef(save);
  saveRef.current = save;

  // Ctrl+S on this page saves the settings (the global handler skips it here).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === "s" && !isAnyModalOpen()) {
        e.preventDefault();
        void saveRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  useEffect(() => {
    const { setLeaveGuard } = useViewStore.getState();
    if (!dirty) {
      setLeaveGuard(null);
      return;
    }
    setLeaveGuard(async () => {
      const pick = await choose({
        title: "设置尚未保存",
        message: "离开前要保存这些修改吗？",
        options: [
          { value: "discard", label: "不保存", variant: "outline" },
          { value: "save", label: "保存" },
        ],
      });
      if (pick === "save") return saveRef.current();
      return pick === "discard";
    });
    return () => setLeaveGuard(null);
  }, [dirty]);

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
          <h1 className="text-lg font-semibold">{VIEW_LABEL.settings}</h1>
          <p className="text-xs text-muted-foreground">配置 AI 模型、输出目录、外部工具与 MCP 服务。</p>
        </div>
        {tab !== "general" && tab !== "mcp" && tab !== "update" && (
          <Button size="sm" onClick={() => void save()} disabled={!dirty && !saved}>
            {saved ? (
              <>
                <Check className="mr-1 h-3.5 w-3.5" /> 已保存
              </>
            ) : (
              <>
                <Save className="mr-1 h-3.5 w-3.5" /> {dirty ? "保存" : "已是最新"}
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
          {tab === "general" && <GeneralSettings />}

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

          {tab === "update" && <UpdatePanel />}

          {tab !== "general" && tab !== "mcp" && tab !== "update" && !inTauri && (
            <p className="text-[11px] text-muted-foreground">
              浏览器预览下设置不会保存，请在应用内配置。
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
