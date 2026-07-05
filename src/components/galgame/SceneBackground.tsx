import { BACKGROUNDS } from "@/assets/galgame";
import type { Mood } from "@/store/galgame";

/** Scene art per mood (base64-inlined via `@/assets/galgame`), under a dark
 * scrim + mood glow + vignette so the sprite and panels stay readable. The
 * gradient sits underneath as a base color while the image decodes. */
const GRADIENT: Record<Mood, string> = {
  neutral: "linear-gradient(to bottom, #1b2233, #141a26, #0b0e15)",
  thinking: "linear-gradient(to bottom, #182a44, #121d30, #0b0e15)",
  happy: "linear-gradient(to bottom, #123a30, #0f2a22, #0b1410)",
  worried: "linear-gradient(to bottom, #3a2030, #281622, #150c12)",
  excited: "linear-gradient(to bottom, #2f1c46, #201538, #100b1c)",
};

const GLOW: Record<Mood, string> = {
  neutral: "rgba(56,189,248,0.20)",
  thinking: "rgba(59,130,246,0.26)",
  happy: "rgba(16,185,129,0.24)",
  worried: "rgba(244,63,94,0.26)",
  excited: "rgba(217,70,239,0.30)",
};

export function SceneBackground({ mood }: { mood: Mood }) {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden">
      <div
        className="absolute inset-0 transition-[background] duration-700"
        style={{ backgroundImage: GRADIENT[mood] }}
      />
      <img
        src={BACKGROUNDS[mood]}
        alt=""
        className="absolute inset-0 h-full w-full object-cover"
      />
      {/* dark scrim: atmospheric + keeps sprite/panels readable */}
      <div className="absolute inset-0 bg-slate-950/45" />
      {/* mood glow */}
      <div
        className="absolute left-1/2 top-1/3 h-[60vh] w-[60vh] -translate-x-1/2 -translate-y-1/2 rounded-full blur-[130px] transition-colors duration-700"
        style={{ backgroundColor: GLOW[mood] }}
      />
      {/* vignette */}
      <div
        className="absolute inset-0"
        style={{
          backgroundImage:
            "radial-gradient(ellipse at center, transparent 45%, rgba(0,0,0,0.55) 100%)",
        }}
      />
    </div>
  );
}
