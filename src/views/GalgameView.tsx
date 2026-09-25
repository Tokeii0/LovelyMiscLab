import { type CSSProperties, useMemo, useRef, useState } from "react";
import {
  FileUp,
  Gamepad2,
  Image as ImageIcon,
  Loader2,
  type LucideIcon,
  Play,
  RotateCcw,
  Type,
  X,
} from "lucide-react";

import type { GalgameChoice } from "@/lib/bindings";
import { Button } from "@/components/ui/button";
import { CharacterSprite } from "@/components/galgame/CharacterSprite";
import { ChoiceList } from "@/components/galgame/ChoiceList";
import { ContentPanel } from "@/components/galgame/ContentPanel";
import { DialogueBox } from "@/components/galgame/DialogueBox";
import { ManualInput } from "@/components/galgame/ManualInput";
import { SceneBackground } from "@/components/galgame/SceneBackground";
import { useTypewriter } from "@/hooks/useTypewriter";
import { inTauri } from "@/lib/devMocks";
import { cn } from "@/lib/utils";
import { useAiStatus } from "@/store/aiStatus";
import { type Mood, useGalgameStore } from "@/store/galgame";
import { toast } from "@/store/toast";
import { useViewStore } from "@/store/view";

const MOODS: Mood[] = ["neutral", "happy", "thinking", "worried", "excited"];
/** Largest file the intro accepts (it travels as a data URL every round). */
const MAX_FILE = 20 * 1024 * 1024;
const asMood = (m: string): Mood => (MOODS.includes(m as Mood) ? (m as Mood) : "neutral");

/** Keyframes for the sprite float, blink, and the select particle burst. */
function StyleTag() {
  return (
    <style>{`
      @keyframes galgame-float { 0%,100%{transform:translateY(0)} 50%{transform:translateY(-12px)} }
      @keyframes galgame-blink { 0%,100%{opacity:1} 50%{opacity:0.15} }
      @keyframes galgame-particle { 0%{transform:translate(0,0) scale(1);opacity:1} 100%{transform:translate(var(--tx),var(--ty)) scale(0.25);opacity:0} }
      @keyframes galgame-ring { 0%{transform:scale(0.2);opacity:0.7} 100%{transform:scale(1.9);opacity:0} }
      .galgame-float{ animation: galgame-float 4.5s ease-in-out infinite; }
      .galgame-blink{ animation: galgame-blink 1.1s steps(1) infinite; }
      .galgame-particle{ animation: galgame-particle 0.7s ease-out forwards; }
      .galgame-ring{ animation: galgame-ring 0.55s ease-out forwards; }
    `}</style>
  );
}

/** A short-lived particle burst at a viewport position, fired on select. */
function ParticleBurst({ x, y }: { x: number; y: number }) {
  const parts = useMemo(
    () =>
      Array.from({ length: 16 }, (_, i) => {
        const ang = (i / 16) * Math.PI * 2 + (Math.random() - 0.5) * 0.6;
        const dist = 36 + Math.random() * 64;
        return {
          tx: Math.cos(ang) * dist,
          ty: Math.sin(ang) * dist,
          size: 4 + Math.random() * 5,
          gold: Math.random() < 0.45,
          delay: Math.random() * 60,
        };
      }),
    []
  );
  return (
    <div className="pointer-events-none fixed z-50" style={{ left: x, top: y }}>
      <span
        className="galgame-ring absolute rounded-full border-2 border-primary"
        style={{ width: 24, height: 24, marginLeft: -12, marginTop: -12 }}
      />
      {parts.map((p, i) => (
        <span
          key={i}
          className="galgame-particle absolute rounded-full"
          style={
            {
              width: p.size,
              height: p.size,
              marginLeft: -p.size / 2,
              marginTop: -p.size / 2,
              background: p.gold ? "#fbbf24" : "var(--color-primary)",
              boxShadow: p.gold ? "0 0 6px #fbbf24" : "0 0 6px var(--color-primary)",
              "--tx": `${p.tx}px`,
              "--ty": `${p.ty}px`,
              animationDelay: `${p.delay}ms`,
            } as CSSProperties
          }
        />
      ))}
    </div>
  );
}

function TopBtn({
  onClick,
  icon: Icon,
  label,
}: {
  onClick: () => void;
  icon: LucideIcon;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-1.5 rounded-lg border border-white/10 bg-slate-900/60 px-3 py-1.5 text-xs text-slate-300 backdrop-blur-md transition-colors hover:border-primary/60 hover:text-white"
    >
      <Icon className="h-3.5 w-3.5" />
      {label}
    </button>
  );
}

function EndingBanner({ ending }: { ending: string }) {
  const good = ending === "good";
  return (
    <div
      className={cn(
        "rounded-xl border px-4 py-2.5 text-center text-sm font-semibold backdrop-blur-md",
        good
          ? "border-emerald-500/50 bg-emerald-500/15 text-emerald-300"
          : "border-rose-500/50 bg-rose-500/15 text-rose-300"
      )}
    >
      {good ? "🎉 通关！flag 到手" : "💀 Bad End —— 这条路走不通，换个方向再试试"}
    </div>
  );
}

type IntroMode = "text" | "image" | "file";

/** Challenge-input screen shown before a story starts. Text is pasted; a file or
 * image is read to a `data:` URL (works in browser + Tauri webview) and handed to
 * the engine, which decodes it to bytes for the identify/extract tools. */
function Intro() {
  const [mode, setMode] = useState<IntroMode>("text");
  const [text, setText] = useState("");
  // The image and file tabs each keep their own pick (a zip must never show up
  // as a "broken image" after switching tabs).
  const [picked, setPicked] = useState<Partial<Record<"image" | "file", { url: string; label: string }>>>({});
  const [brief, setBrief] = useState("");
  const start = useGalgameStore((s) => s.start);
  const llmReady = useAiStatus((s) => s.llm);
  const setView = useViewStore((s) => s.setView);

  const readFile = (slot: "image" | "file", f: File | undefined) => {
    if (!f) return;
    if (f.size > MAX_FILE) {
      toast.error("文件太大", { detail: `故事模式最多处理 ${MAX_FILE / 1024 / 1024} MB 的文件` });
      return;
    }
    const label = `${f.name} · ${(f.size / 1024).toFixed(1)} KB`;
    const r = new FileReader();
    r.onload = () => setPicked((p) => ({ ...p, [slot]: { url: r.result as string, label } }));
    r.onerror = () => toast.error("读取文件失败", { error: r.error });
    r.readAsDataURL(f);
  };

  const current = mode === "text" ? undefined : picked[mode];
  const payload = mode === "text" ? text : (current?.url ?? "");
  const canStart = mode === "text" ? !!text.trim() : !!current;
  const tabs: { id: IntroMode; label: string; icon: LucideIcon }[] = [
    { id: "text", label: "文本 / 密文", icon: Type },
    { id: "image", label: "图片", icon: ImageIcon },
    { id: "file", label: "文件", icon: FileUp },
  ];

  return (
    <div className="relative flex h-full items-center justify-center overflow-hidden">
      <StyleTag />
      <SceneBackground mood="neutral" />
      <div className="relative z-10 w-[560px] max-w-[92vw] rounded-2xl border border-white/10 bg-slate-900/85 p-7 shadow-2xl backdrop-blur-md">
        <div className="mb-5 flex items-center gap-3">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/15 text-primary">
            <Gamepad2 className="h-6 w-6" />
          </div>
          <div>
            <h2 className="text-lg font-semibold text-slate-100">故事模式</h2>
            <p className="text-xs text-slate-400">
              把解题变成一场 galgame——傲娇搭档 Misca 嘴上嫌弃，其实一步步陪你把题啃下来。
            </p>
          </div>
        </div>

        <div className="mb-3 flex gap-1 rounded-lg border border-white/10 bg-slate-950/40 p-1">
          {tabs.map((t) => (
            <button
              key={t.id}
              onClick={() => setMode(t.id)}
              className={cn(
                "flex flex-1 items-center justify-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
                mode === t.id ? "bg-primary text-white" : "text-slate-300 hover:text-white"
              )}
            >
              <t.icon className="h-3.5 w-3.5" />
              {t.label}
            </button>
          ))}
        </div>

        {mode === "text" && (
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="把题目数据粘进来（一段文本 / 密文 / 编码……），然后开始你的解题冒险。"
            className="h-40 w-full resize-none rounded-lg border border-white/10 bg-slate-950/60 p-3 font-mono text-sm text-slate-100 outline-none placeholder:text-slate-500 focus:border-primary focus:ring-2 focus:ring-primary/30"
          />
        )}

        {mode === "image" && (
          <div className="flex h-40 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-white/15 bg-slate-950/60 p-3">
            {picked.image ? (
              <img
                src={picked.image.url}
                alt=""
                className="max-h-24 rounded border border-white/10 bg-white object-contain"
              />
            ) : (
              <ImageIcon className="h-8 w-8 text-slate-500" />
            )}
            <label className="cursor-pointer rounded-lg border border-white/10 bg-slate-800/60 px-3 py-1.5 text-xs text-slate-200 transition-colors hover:border-primary/60">
              选择图片
              <input
                type="file"
                accept="image/*"
                className="hidden"
                onChange={(e) => readFile("image", e.target.files?.[0])}
              />
            </label>
            {picked.image && (
              <span className="max-w-full truncate text-[11px] text-slate-400">{picked.image.label}</span>
            )}
          </div>
        )}

        {mode === "file" && (
          <div className="flex h-40 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-white/15 bg-slate-950/60 p-3">
            <FileUp className="h-8 w-8 text-slate-500" />
            <label className="cursor-pointer rounded-lg border border-white/10 bg-slate-800/60 px-3 py-1.5 text-xs text-slate-200 transition-colors hover:border-primary/60">
              选择文件
              <input
                type="file"
                className="hidden"
                onChange={(e) => readFile("file", e.target.files?.[0])}
              />
            </label>
            {picked.file ? (
              <span className="max-w-full truncate text-[11px] text-slate-300">{picked.file.label}</span>
            ) : (
              <span className="px-4 text-center text-[11px] text-slate-500">
                图片 / 压缩包 / 任意文件都行——Misca 会先识别类型、抽字符串、查隐写，再进入解码。
              </span>
            )}
          </div>
        )}

        <textarea
          value={brief}
          onChange={(e) => setBrief(e.target.value)}
          placeholder="可选 · 题干/提示：把题目描述也写上，Misca 会参考它（例：附件是张 PNG，flag 藏在 LSB；压缩包密码是出题人生日）"
          className="mt-3 h-16 w-full resize-none rounded-lg border border-white/10 bg-slate-950/60 p-3 text-sm text-slate-100 outline-none placeholder:text-slate-500 focus:border-primary focus:ring-2 focus:ring-primary/30"
        />

        <div className="mt-4 flex items-center justify-between gap-3">
          <p className="text-[11px] leading-tight text-slate-400">
            {!inTauri ? (
              "浏览器预览使用模拟剧情；桌面应用内为真实解题。"
            ) : llmReady === false ? (
              <>
                需要先配置 AI 文本模型。
                <button className="ml-1 text-sky-300 underline" onClick={() => setView("settings")}>
                  去设置
                </button>
              </>
            ) : (
              "Misca 由你在设置中配置的 AI 文本模型驱动。"
            )}
          </p>
          <Button disabled={!canStart} onClick={() => start(payload, mode, brief)}>
            <Play className="h-4 w-4" />
            开始解题
          </Button>
        </div>
      </div>
    </div>
  );
}

/** The playing screen: Misca (left) + content/dialogue/choices/input (right). */
function Story() {
  const speaker = useGalgameStore((s) => s.speaker);
  const mood = useGalgameStore((s) => s.mood);
  const narration = useGalgameStore((s) => s.narration);
  const choices = useGalgameStore((s) => s.choices);
  const busy = useGalgameStore((s) => s.busy);
  const ending = useGalgameStore((s) => s.ending);
  const error = useGalgameStore((s) => s.error);
  const errorCode = useGalgameStore((s) => s.errorCode);
  const lastOutputs = useGalgameStore((s) => s.lastOutputs);
  const challenge = useGalgameStore((s) => s.challenge);
  const workData = useGalgameStore((s) => s.workData);
  const sceneSeed = useGalgameStore((s) => s.sceneSeed);
  const awaitingPassword = useGalgameStore((s) => s.awaitingPassword);
  const pick = useGalgameStore((s) => s.pick);
  const retry = useGalgameStore((s) => s.retry);
  const cancel = useGalgameStore((s) => s.cancel);
  const reset = useGalgameStore((s) => s.reset);
  const setView = useViewStore((s) => s.setView);
  // An image the last tool produced (it became the working data).
  const resultImage = workData !== challenge && workData.startsWith("data:image") ? workData : null;

  const { shown, done, skip } = useTypewriter(narration);
  const m = asMood(mood);

  // Select particle bursts.
  const [bursts, setBursts] = useState<{ id: number; x: number; y: number }[]>([]);
  const burstId = useRef(0);
  const fire = (x: number, y: number) => {
    const id = (burstId.current += 1);
    setBursts((b) => [...b, { id, x, y }]);
    window.setTimeout(() => setBursts((b) => b.filter((z) => z.id !== id)), 800);
  };
  const handlePick = (c: GalgameChoice, at?: { x: number; y: number }) => {
    if (at) fire(at.x, at.y);
    pick(c);
  };

  return (
    <div className="relative flex h-full flex-col overflow-hidden">
      <StyleTag />
      <SceneBackground mood={m} />
      {bursts.map((b) => (
        <ParticleBurst key={b.id} x={b.x} y={b.y} />
      ))}

      {/* top bar */}
      <div className="relative z-20 flex items-center justify-between p-3">
        <div className="flex items-center gap-1.5 rounded-lg border border-white/10 bg-slate-900/60 px-3 py-1.5 text-xs text-slate-100 backdrop-blur-md">
          <Gamepad2 className="h-4 w-4 text-primary" /> 故事模式
        </div>
        <div className="flex items-center gap-2">
          <TopBtn onClick={reset} icon={RotateCcw} label="重新开始" />
          <TopBtn onClick={() => setView("canvas")} icon={X} label="退出" />
        </div>
      </div>

      {/* body: Misca (left) | content + dialogue + choices (right) */}
      <div className="relative z-10 flex min-h-0 flex-1 gap-2 px-3 pb-3">
        {/* left: character */}
        <div className="relative flex w-[34%] max-w-[420px] shrink-0 items-end justify-center">
          <CharacterSprite mood={m} seed={sceneSeed} />
        </div>

        {/* right column */}
        <div className="relative flex min-w-0 flex-1 flex-col gap-3">
          <ContentPanel outputs={lastOutputs} challenge={challenge} image={resultImage} />

          <div className="space-y-3">
            {error && (
              <div className="flex items-start gap-2 rounded-lg border border-rose-500/50 bg-rose-500/15 px-3 py-2 text-xs text-rose-200">
                <span className="min-w-0 flex-1 break-words">{error}</span>
                {errorCode === "ai_config" ? (
                  <button
                    onClick={() => setView("settings")}
                    className="shrink-0 rounded border border-rose-300/40 px-2 py-0.5 hover:bg-rose-500/20"
                  >
                    去设置
                  </button>
                ) : (
                  <button
                    onClick={retry}
                    className="shrink-0 rounded border border-rose-300/40 px-2 py-0.5 hover:bg-rose-500/20"
                  >
                    重试
                  </button>
                )}
              </div>
            )}
            {ending && <EndingBanner ending={ending} />}
            <DialogueBox speaker={speaker} text={shown} done={done} thinking={busy} onSkip={skip} />

            {busy ? (
              <div className="flex items-center justify-center gap-3 py-1 text-sm text-slate-300">
                <Loader2 className="h-4 w-4 animate-spin" /> 生成中…
                <button
                  onClick={cancel}
                  className="rounded border border-white/15 px-2 py-0.5 text-xs text-slate-300 hover:border-rose-400/60 hover:text-rose-200"
                >
                  停止
                </button>
              </div>
            ) : (
              <>
                {/* Choices appear once the line has finished typing (click the box to skip). */}
                {choices.length > 0 && done && (
                  <ChoiceList choices={choices} disabled={busy} onPick={handlePick} />
                )}
                {ending && choices.length === 0 && (
                  <Button onClick={reset} className="w-full">
                    <RotateCcw className="h-4 w-4" /> 再来一局
                  </Button>
                )}
              </>
            )}

            <ManualInput
              disabled={busy}
              awaitingPassword={awaitingPassword}
              onSubmit={(t, at) => {
                if (at) fire(at.x, at.y);
                pick({ text: t });
              }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

/** 故事模式 (galgame): solve CTF challenges as a visual novel. */
export function GalgameView() {
  const started = useGalgameStore((s) => s.started);
  return started ? <Story /> : <Intro />;
}
