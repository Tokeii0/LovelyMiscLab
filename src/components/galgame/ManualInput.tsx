import { useRef, useState } from "react";
import { Send } from "lucide-react";

import { cn } from "@/lib/utils";

/** Free-text directive box under the preset choices. What the player types is
 * sent as a solving instruction (Misca builds + runs a pipeline for it). Reports
 * the send button's position so the view can fire a particle burst there. Pinned
 * to a dark-glass look (light text) for the always-dark scene. */
export function ManualInput({
  disabled,
  onSubmit,
}: {
  disabled?: boolean;
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
          if (e.key === "Enter") {
            e.preventDefault();
            send();
          }
        }}
        disabled={disabled}
        placeholder="输入你的解题思路，让 Misca 照着做…（例：先用正则抠出 base64 再循环解码）"
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
