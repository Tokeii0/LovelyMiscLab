import { useState } from "react";
import { Bot, Loader2, RefreshCw, Sparkles, Wand2, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { Template } from "@/lib/templates";
import { cn } from "@/lib/utils";
import { loadTemplate } from "@/flow/loadTemplate";
import { buildGraph } from "@/flow/runner";
import { useAiStore } from "@/store/ai";
import { useRunStore } from "@/store/run";
import { useViewStore } from "@/store/view";

type Mode = "generate" | "explain" | "repair";

const EXAMPLES = [
  "把这段套娃 Base64 一直解码，直到出现 flag 再提取出来",
  "对输入文本计算 SHA256",
  "AES-CBC 解密：给定密文 Hex、密钥、IV",
  "生成一个二维码，内容是 flag{ai_made_this}",
  "把 GBK 乱码字节还原成中文",
];

const MODES: { mode: Mode; label: string; icon: typeof Sparkles; hint: string }[] = [
  { mode: "generate", label: "生成", icon: Sparkles, hint: "从一句话创建节点图" },
  { mode: "explain", label: "解释", icon: Bot, hint: "解释当前画布的数据流" },
  { mode: "repair", label: "修复", icon: RefreshCw, hint: "根据当前图和错误重建流程" },
];

export function AiGenerateDialog() {
  const open = useAiStore((s) => s.open);
  const setOpen = useAiStore((s) => s.setOpen);
  const setView = useViewStore((s) => s.setView);
  const lastError = useRunStore((s) => s.lastError);
  const [mode, setMode] = useState<Mode>("generate");
  const [prompt, setPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [answer, setAnswer] = useState("");

  if (!open) return null;

  const close = () => {
    if (!loading) setOpen(false);
  };

  const loadGenerated = (g: Awaited<ReturnType<typeof api.generateWorkflow>>) => {
    const template: Template = {
      id: "ai-generated",
      name: mode === "repair" ? "AI 修复流程" : "AI 生成流程",
      description: g.notes,
      category: "AI",
      icon: Sparkles,
      nodes: g.nodes,
      edges: g.edges,
    };
    const loaded = loadTemplate(template);
    if (loaded === 0) {
      setError("AI 返回的流程为空或节点无法识别，请换个描述再试。");
      return;
    }
    setOpen(false);
    setView("canvas");
    setPrompt("");
    setAnswer("");
  };

  const run = async () => {
    if (loading) return;
    if (!inTauri) {
      setError("浏览器预览无法调用 AI，请在桌面应用内使用，并先到设置里配置文本模型。");
      return;
    }
    if (mode === "generate" && !prompt.trim()) return;

    setLoading(true);
    setError("");
    setAnswer("");
    try {
      if (mode === "explain") {
        const result = await api.explainWorkflow(buildGraph(), prompt.trim());
        setAnswer(result.text);
        return;
      }
      if (mode === "repair") {
        const graph = buildGraph();
        const repairPrompt = [
          "请根据下面的 LovelyMiscLab 当前流程和错误信息，重新生成一张更可靠的节点图。",
          prompt.trim() ? `用户补充要求：${prompt.trim()}` : "",
          lastError ? `最近错误：${lastError}` : "最近错误：无显式错误，请优化流程结构和参数。",
          `当前流程 JSON：${JSON.stringify(graph)}`,
        ]
          .filter(Boolean)
          .join("\n\n");
        loadGenerated(await api.generateWorkflow(repairPrompt));
        return;
      }
      loadGenerated(await api.generateWorkflow(prompt.trim()));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const buttonText =
    mode === "generate" ? "生成流程" : mode === "explain" ? "解释流程" : "修复流程";

  return (
    <div
      className="fixed inset-0 z-[75] flex items-center justify-center bg-black/50 p-4"
      onClick={close}
    >
      <div
        className="flex max-h-[88vh] w-[620px] max-w-[95vw] flex-col rounded-lg border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border p-4">
          <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Wand2 className="h-5 w-5" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="text-base font-semibold">AI 工作流助手</div>
            <div className="text-xs text-muted-foreground">生成、解释或修复当前节点图</div>
          </div>
          <button onClick={close} className="text-muted-foreground hover:text-foreground">
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="border-b border-border p-3">
          <div className="grid grid-cols-3 gap-2">
            {MODES.map((m) => {
              const Icon = m.icon;
              return (
                <button
                  key={m.mode}
                  onClick={() => {
                    setMode(m.mode);
                    setError("");
                    setAnswer("");
                  }}
                  className={cn(
                    "rounded-md border px-3 py-2 text-left transition-colors",
                    mode === m.mode
                      ? "border-primary bg-primary/10 text-primary"
                      : "border-border hover:bg-accent"
                  )}
                >
                  <div className="flex items-center gap-1.5 text-sm font-medium">
                    <Icon className="h-3.5 w-3.5" />
                    {m.label}
                  </div>
                  <div className="mt-0.5 text-[10px] text-muted-foreground">{m.hint}</div>
                </button>
              );
            })}
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          <textarea
            autoFocus
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            rows={mode === "explain" ? 3 : 4}
            placeholder={
              mode === "generate"
                ? "例如：把这段套娃 base64 一直解码，直到出现 flag，再提取出来"
                : mode === "explain"
                  ? "可选：告诉 AI 你最关心哪部分，例如失败点、参数含义或后续优化"
                  : "可选：描述希望如何修复，例如保留文件导入、改成自动解码、补 text_output"
            }
            className="w-full resize-none rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
            onKeyDown={(e) => {
              if ((e.ctrlKey || e.metaKey) && e.key === "Enter") void run();
            }}
          />
          {mode === "generate" && (
            <div className="mt-2 flex flex-wrap gap-1.5">
              {EXAMPLES.map((ex) => (
                <button
                  key={ex}
                  onClick={() => setPrompt(ex)}
                  className="rounded-full bg-secondary px-2.5 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                >
                  {ex}
                </button>
              ))}
            </div>
          )}
          {mode === "repair" && lastError && (
            <div className="mt-3 rounded-lg bg-destructive/10 p-2.5 text-xs text-destructive">
              最近错误：{lastError}
            </div>
          )}
          {error && (
            <div className="mt-3 whitespace-pre-wrap rounded-lg bg-destructive/10 p-2.5 text-xs text-destructive">
              {error}
            </div>
          )}
          {answer && (
            <pre className="mt-3 max-h-72 whitespace-pre-wrap rounded-lg border border-border bg-background p-3 text-xs leading-relaxed">
              {answer}
            </pre>
          )}
          {!inTauri && (
            <div className="mt-3 text-[11px] text-muted-foreground">
              提示：需要在桌面应用内，并在“设置 → AI 模型”配置文本模型后使用。
            </div>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-border p-4">
          <span className="text-[11px] text-muted-foreground">Ctrl + Enter 执行</span>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => setOpen(false)} disabled={loading}>
              取消
            </Button>
            <Button size="sm" onClick={run} disabled={loading || (mode === "generate" && !prompt.trim())}>
              {loading ? (
                <>
                  <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> 处理中...
                </>
              ) : (
                <>
                  <Sparkles className="mr-1 h-3.5 w-3.5" /> {buttonText}
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
