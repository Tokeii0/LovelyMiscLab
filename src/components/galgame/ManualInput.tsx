import { useRef, useState } from "react";
import { Send } from "lucide-react";

import { cn } from "@/lib/utils";

/** Free-text box under the preset choices. What the player types goes to Misca
 * as a hint for the next step (she reacts and offers matching choices; it isn't
 * executed directly) — or, while an encrypted archive waits, is tried as its
 * password. Reports the send button's position so the view can fire a particle
 * burst there. Pinned to a dark-glass look (light text) for the always-dark scene. */
export function ManualInput({
  disabled,
  awaitingPassword,
  onSubmit,
}: {
  disabled?: boolean;
  awaitingPassword?: boolean;
  onSubmit: (text: string, at?: { x: number; y: number }) => void;
}) {
  const [text, setText] = useState("");
  const btnRef = useRef<HTMLButtonElement>(null);
  const send = () => {
    const t = text.trim();
    if (!t || disabled) return;
    const r = btnRef.current?.getBoundingClientRect();
    onSubmit(t, r ? { x: r.left + r.width / 2, y: r.top + r.height / 2 } : undefined);
    setText("");
  };
  return (
    <div className="pointer-events-auto flex items-center gap-2 rounded-xl border border-white/10 bg-slate-900/60 px-2 py-1.5 backdrop-blur-md focus-within:border-primary">
      <input
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          // Enter while an IME is composing picks a candidate, not "send".
          if (e.key === "Enter" && !e.nativeEvent.isComposing) {
            e.preventDefault();
            send();
          }
        }}
        disabled={disabled}
        placeholder={
          awaitingPassword
            ? "输入压缩包密码（按原样尝试）…"
            : "告诉 Misca 你的想法，她会据此给出下一步选项…（例：像是套了几层 base64）"
        }
        className="min-w-0 flex-1 bg-transparent px-2 py-1 text-sm text-slate-100 outline-none placeholder:text-slate-400 disabled:opacity-50"
      />
      <button
        ref={btnRef}
        onClick={send}
        disabled={disabled || !text.trim()}
        className={cn(
          "flex h-8 shrink-0 items-center gap-1 rounded-lg px-3 text-xs font-medium transition-colors",
          "bg-primary text-white hover:bg-primary/90 disabled:pointer-events-none disabled:opacity-40"
        )}
      >
        <Send className="h-3.5 w-3.5" /> 发送
      </button>
    </div>
  );
}
