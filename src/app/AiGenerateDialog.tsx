import { useState } from "react";
import { Loader2, Sparkles, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { Template } from "@/lib/templates";
import { loadTemplate } from "@/flow/loadTemplate";
import { useAiStore } from "@/store/ai";
import { useViewStore } from "@/store/view";

const EXAMPLES = [
  "把这段套娃 Base64 一直解码，直到出现 flag 再提取出来",
  "对输入文本计算 SHA256",
  "AES-CBC 解密：给定密文(Hex)、密钥、IV",
  "生成一个二维码，内容是 flag{ai_made_this}",
  "把 GBK 乱码字节还原成中文",
];

export function AiGenerateDialog() {
  const open = useAiStore((s) => s.open);
  const setOpen = useAiStore((s) => s.setOpen);
  const setView = useViewStore((s) => s.setView);
  const [prompt, setPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  if (!open) return null;

  const close = () => {
    if (!loading) setOpen(false);
  };

  const generate = async () => {
    if (!prompt.trim() || loading) return;
    if (!inTauri) {
      setError("浏览器预览无法调用 AI，请在应用中使用（并在「设置 → AI 模型」配置文本模型）。");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const g = await api.generateWorkflow(prompt.trim());
      const template: Template = {
        id: "ai-generated",
        name: "AI 生成流程",
        description: g.notes,
        category: "AI",
        icon: Sparkles,
        nodes: g.nodes,
        edges: g.edges,
      };
      const loaded = loadTemplate(template);
      if (loaded === 0) {
        setError("AI 生成的流程为空或节点无法识别，请换个描述再试。");
        return;
      }
      setOpen(false);
      setView("canvas");
      setPrompt("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[75] flex items-center justify-center bg-black/50 p-4"
      onClick={close}
    >
      <div
        className="w-[560px] max-w-[95vw] rounded-xl border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border p-4">
          <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Sparkles className="h-5 w-5" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="text-base font-semibold">AI 生成流程</div>
            <div className="text-xs text-muted-foreground">用一句话描述任务，自动搭建节点流程图</div>
          </div>
          <button onClick={close} className="text-muted-foreground hover:text-foreground">
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="p-4">
          <textarea
            autoFocus
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            rows={4}
            placeholder="例如：把这段套娃 base64 一直解码，直到出现 flag，再提取出来"
            className="w-full resize-none rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
            onKeyDown={(e) => {
              if ((e.ctrlKey || e.metaKey) && e.key === "Enter") void generate();
            }}
          />
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
          {error && (
            <div className="mt-3 whitespace-pre-wrap rounded-lg bg-destructive/10 p-2.5 text-xs text-destructive">
              {error}
            </div>
          )}
          {!inTauri && (
            <div className="mt-3 text-[11px] text-muted-foreground">
              提示：需在应用内、并在「设置 → AI 模型」配置文本模型后使用。
            </div>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-border p-4">
          <span className="text-[11px] text-muted-foreground">Ctrl + Enter 生成</span>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => setOpen(false)} disabled={loading}>
              取消
            </Button>
            <Button size="sm" onClick={generate} disabled={loading || !prompt.trim()}>
              {loading ? (
                <>
                  <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" /> 生成中…
                </>
              ) : (
                <>
                  <Sparkles className="mr-1 h-3.5 w-3.5" /> 生成流程
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
