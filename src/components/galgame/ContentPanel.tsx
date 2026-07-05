import { useState } from "react";
import { ChevronDown, ChevronRight, Terminal } from "lucide-react";

/** The "screen" above the dialogue: the current engine run-result (what Misca is
 * reacting to), the pipeline note, and a collapsible raw-challenge view. Styled
 * as a solid dark monitor so mono text stays readable over the mood gradient. */
export function ContentPanel({
  outputs,
  challenge,
  notes,
}: {
  outputs: string | null;
  challenge: string;
  notes?: string | null;
}) {
  const [showChallenge, setShowChallenge] = useState(false);
  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-white/10 bg-[#0b0f18]/92 shadow-inner backdrop-blur-md">
      <div className="flex items-center gap-2 border-b border-white/10 px-3 py-2 text-xs font-semibold text-slate-100">
        <Terminal className="h-3.5 w-3.5 shrink-0 text-primary" />
        运行结果
        {notes && (
          <span className="truncate text-[11px] font-normal text-slate-400">· {notes}</span>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-3">
        {outputs ? (
          <pre className="whitespace-pre-wrap break-words font-mono text-[13px] leading-relaxed text-emerald-300">
            {outputs}
          </pre>
        ) : (
          <div className="flex h-full items-center justify-center px-6 text-center text-xs text-slate-400">
            还没有运行结果。选一个带 ⚙ 的选项后，Misca 会把当前数据交给对应节点执行。
          </div>
        )}
      </div>

      <button
        onClick={() => setShowChallenge((v) => !v)}
        className="flex items-center gap-1 border-t border-white/10 px-3 py-1.5 text-[11px] text-slate-400 transition-colors hover:text-slate-100"
      >
        {showChallenge ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        题目原文
      </button>
      {showChallenge && (
        <div className="max-h-28 overflow-auto border-t border-white/10 bg-black/40 p-3">
          <pre className="whitespace-pre-wrap break-words font-mono text-xs leading-relaxed text-slate-200">
            {challenge || "（无文本）"}
          </pre>
        </div>
      )}
    </div>
  );
}
