/** The visual-novel text box: name plate + typewriter narration. Click anywhere
 * to skip the typewriter to the end. Pinned to a dark-glass look (light text) so
 * it stays readable on the always-dark scene regardless of the app theme. */
export function DialogueBox({
  speaker,
  text,
  done,
  thinking,
  onSkip,
}: {
  speaker: string;
  text: string;
  done: boolean;
  thinking?: boolean;
  onSkip: () => void;
}) {
  return (
    <div
      onClick={onSkip}
      className="pointer-events-auto relative cursor-pointer select-none rounded-2xl border border-white/10 bg-slate-900/75 p-5 pt-6 shadow-2xl backdrop-blur-md"
    >
      {/* name plate */}
      <div className="absolute -top-4 left-5 rounded-lg border border-primary/50 bg-primary px-4 py-1 text-sm font-semibold text-white shadow-lg">
        {speaker || "Misca"}
      </div>

      <p className="min-h-[4.5rem] text-[15px] leading-relaxed text-slate-100">
        {thinking ? (
          <span className="inline-flex items-center gap-2 text-slate-400">
            <span className="galgame-blink">Misca 正在思考</span>
            <span className="galgame-blink">…</span>
          </span>
        ) : (
          <>
            {text}
            {!done && <span className="galgame-blink ml-0.5 text-sky-300">▍</span>}
          </>
        )}
      </p>

      {done && !thinking && (
        <div className="mt-1 text-right text-sky-300/80">
          <span className="galgame-blink text-xs">▼</span>
        </div>
      )}
    </div>
  );
}
