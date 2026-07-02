import { useEffect, useState } from "react";
import { Check, Copy, Loader2, RefreshCw, Server, ShieldAlert } from "lucide-react";

import { Button } from "@/components/ui/button";
import { api, type McpSettings, type McpStatus } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";

type ClientId = "claude-code" | "cursor" | "codex" | "other";

const CLIENTS: { id: ClientId; label: string; note: string }[] = [
  { id: "claude-code", label: "Claude Code", note: "在终端运行此命令即可添加（HTTP 传输）。" },
  { id: "cursor", label: "Cursor", note: "写入 ~/.cursor/mcp.json（或项目内 .cursor/mcp.json）。" },
  { id: "codex", label: "Codex", note: "写入 ~/.codex/config.toml，用 mcp-remote 把 HTTP 桥接成 stdio。" },
  { id: "other", label: "其他 MCP", note: "Claude Desktop / Cline 等仅支持 stdio 的客户端，用 mcp-remote 桥接。" },
];

/** Ready-to-copy config for connecting each client to this local server. */
function clientSnippet(id: ClientId, endpoint: string, token: string): string {
  switch (id) {
    case "claude-code":
      return `claude mcp add --transport http lovelymisclab ${endpoint} \\\n  --header "Authorization: Bearer ${token}"`;
    case "cursor":
      return `// ~/.cursor/mcp.json\n{\n  "mcpServers": {\n    "lovelymisclab": {\n      "url": "${endpoint}",\n      "headers": { "Authorization": "Bearer ${token}" }\n    }\n  }\n}`;
    case "codex":
      return `# ~/.codex/config.toml\n[mcp_servers.lovelymisclab]\ncommand = "npx"\nargs = ["-y", "mcp-remote", "${endpoint}", "--header", "Authorization: Bearer ${token}"]`;
    case "other":
      return `// 通用 mcpServers（stdio 客户端经 mcp-remote 桥接）\n{\n  "mcpServers": {\n    "lovelymisclab": {\n      "command": "npx",\n      "args": ["-y", "mcp-remote", "${endpoint}",\n               "--header", "Authorization: Bearer ${token}"]\n    }\n  }\n}`;
  }
}

/** Settings panel for the embedded MCP server: start/stop, port, token, endpoint. */
export function McpPanel() {
  const [cfg, setCfg] = useState<McpSettings>({ enabled: false, port: 8765, token: null, bindAll: false });
  const [status, setStatus] = useState<McpStatus | null>(null);
  const [available, setAvailable] = useState(true);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const [client, setClient] = useState<ClientId>("claude-code");

  const refresh = async () => {
    const [c, st] = await Promise.all([api.mcpGetConfig(), api.mcpStatus()]);
    setCfg(c);
    setStatus(st);
  };

  useEffect(() => {
    if (!inTauri) {
      setAvailable(false);
      return;
    }
    refresh().catch(() => setAvailable(false));
  }, []);

  if (!available) {
    return (
      <section>
        <h2 className="mb-2 text-sm font-semibold">MCP 服务（供 AI 调用）</h2>
        <div className="rounded-xl border border-border bg-card p-4 text-[11px] text-muted-foreground">
          当前构建未启用 MCP（需以 <code>--features mcp</code> 构建），或不在应用内运行。
        </div>
      </section>
    );
  }

  const running = status?.running ?? false;
  const endpoint =
    status?.endpoint ?? `http://${cfg.bindAll ? "0.0.0.0" : "127.0.0.1"}:${cfg.port}/mcp`;

  const start = async () => {
    setBusy(true);
    try {
      await api.mcpStart();
      await refresh();
    } catch (e) {
      console.error("mcpStart failed", e);
    } finally {
      setBusy(false);
    }
  };
  const stop = async () => {
    setBusy(true);
    try {
      await api.mcpStop();
      await refresh();
    } catch (e) {
      console.error("mcpStop failed", e);
    } finally {
      setBusy(false);
    }
  };
  const persist = async (next: McpSettings) => {
    setCfg(next);
    try {
      await api.mcpSetConfig(next);
    } catch (e) {
      console.error("mcpSetConfig failed", e);
    }
  };
  const regen = () => void persist({ ...cfg, token: crypto.randomUUID().replace(/-/g, "") });
  const copy = (label: string, text: string) => {
    void navigator.clipboard.writeText(text);
    setCopied(label);
    setTimeout(() => setCopied(null), 1200);
  };

  return (
    <section>
      <div className="mb-2 flex items-center gap-2">
        <Server className="h-4 w-4 text-muted-foreground" />
        <h2 className="text-sm font-semibold">MCP 服务（供 AI 调用）</h2>
        <span
          className={`ml-1 rounded-full px-2 py-0.5 text-[10px] ${
            running ? "bg-green-500/15 text-green-600" : "bg-muted text-muted-foreground"
          }`}
        >
          {running ? "运行中" : "已停止"}
        </span>
      </div>

      <div className="space-y-3 rounded-xl border border-border bg-card p-4">
        <div className="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 p-2.5 text-[11px] text-amber-700 dark:text-amber-400">
          <ShieldAlert className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>
            启用后，持有下方令牌的 AI 客户端可运行节点/脚本、读取与修改你的画布。默认仅监听本机，请勿把令牌泄露给他人。
          </span>
        </div>

        <div className="flex items-center gap-2">
          {running ? (
            <Button size="sm" variant="outline" onClick={stop} disabled={busy}>
              {busy && <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />}停止
            </Button>
          ) : (
            <Button size="sm" onClick={start} disabled={busy}>
              {busy && <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />}启动
            </Button>
          )}
          <input
            readOnly
            value={endpoint}
            className="flex-1 rounded-md border border-input bg-background px-2.5 py-1.5 font-mono text-xs"
          />
          <button
            onClick={() => copy("endpoint", endpoint)}
            className="rounded-md border border-border p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
            title="复制端点"
          >
            {copied === "endpoint" ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          </button>
        </div>

        <div className="flex flex-wrap items-center gap-4">
          <label className="flex items-center gap-2 text-[11px] text-muted-foreground">
            端口
            <input
              type="number"
              value={cfg.port}
              disabled={running}
              onChange={(e) => void persist({ ...cfg, port: Number(e.target.value) || 8765 })}
              className="w-24 rounded-md border border-input bg-background px-2 py-1 text-xs disabled:opacity-50"
            />
          </label>
          <label className="flex items-center gap-2 text-[11px] text-muted-foreground">
            <input
              type="checkbox"
              checked={cfg.bindAll}
              disabled={running}
              onChange={(e) => void persist({ ...cfg, bindAll: e.target.checked })}
            />
            允许局域网访问 (0.0.0.0，谨慎)
          </label>
          {running && <span className="text-[10px] text-muted-foreground">停止后可修改端口/令牌</span>}
        </div>

        <div>
          <div className="mb-1 text-[11px] text-muted-foreground">访问令牌 (Bearer)</div>
          <div className="flex items-center gap-2">
            <input
              readOnly
              value={cfg.token ?? "（启动后自动生成）"}
              className="flex-1 rounded-md border border-input bg-background px-2.5 py-1.5 font-mono text-xs"
            />
            <button
              onClick={() => cfg.token && copy("token", cfg.token)}
              disabled={!cfg.token}
              className="rounded-md border border-border p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40"
              title="复制令牌"
            >
              {copied === "token" ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            </button>
            <Button variant="outline" size="sm" onClick={regen} disabled={running}>
              <RefreshCw className="mr-1 h-3.5 w-3.5" /> 重置
            </Button>
          </div>
        </div>

        <div>
          <div className="mb-1.5 text-[11px] text-muted-foreground">在 AI 客户端中连接</div>
          <div className="mb-2 flex flex-wrap gap-1">
            {CLIENTS.map((c) => (
              <button
                key={c.id}
                onClick={() => setClient(c.id)}
                className={`rounded-md px-2 py-1 text-[11px] transition-colors ${
                  client === c.id
                    ? "bg-primary text-primary-foreground"
                    : "border border-border text-muted-foreground hover:bg-accent hover:text-foreground"
                }`}
              >
                {c.label}
              </button>
            ))}
          </div>
          <div className="relative">
            <pre className="max-h-52 overflow-auto whitespace-pre rounded-md border border-border bg-background p-3 pr-10 font-mono text-[11px] leading-relaxed">
              {clientSnippet(client, endpoint, cfg.token ?? "<启动后生成的令牌>")}
            </pre>
            <button
              onClick={() => copy("snippet", clientSnippet(client, endpoint, cfg.token ?? "<TOKEN>"))}
              className="absolute right-2 top-2 rounded-md border border-border bg-card p-1.5 text-muted-foreground hover:text-foreground"
              title="复制配置"
            >
              {copied === "snippet" ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            </button>
          </div>
          <p className="mt-1 text-[10px] text-muted-foreground">
            {CLIENTS.find((c) => c.id === client)?.note}
            {!cfg.token && " 先点「启动」生成令牌。"}
          </p>
        </div>
      </div>
    </section>
  );
}
