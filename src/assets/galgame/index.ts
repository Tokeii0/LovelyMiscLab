// Galgame art imported as modules so Vite inlines each as a base64 data URI at
// build time (see `assetsInlineLimit` in vite.config.ts). This keeps the built
// bundle self-contained — no separate image files ship alongside it. In dev the
// same imports resolve to normal URLs (only production `build` inlines).
// Art is WebP (with alpha) to keep the inlined payload small.
import type { Mood } from "@/store/galgame";

import neutral from "./misca-neutral.webp";
import neutral2 from "./misca-neutral-2.webp";
import happy from "./misca-happy.webp";
import happy2 from "./misca-happy-2.webp";
import thinking from "./misca-thinking.webp";
import thinking2 from "./misca-thinking-2.webp";
import worried from "./misca-worried.webp";
import worried2 from "./misca-worried-2.webp";
import excited from "./misca-excited.webp";
import excited2 from "./misca-excited-2.webp";
import bg1 from "./bg-1.webp";
import bg2 from "./bg-2.webp";
import bg3 from "./bg-3.webp";
import bg4 from "./bg-4.webp";
import bg5 from "./bg-5.webp";

/** Two art variants per mood (a per-turn seed picks one). */
export const SPRITES: Record<Mood, string[]> = {
  neutral: [neutral, neutral2],
  happy: [happy, happy2],
  thinking: [thinking, thinking2],
  worried: [worried, worried2],
  excited: [excited, excited2],
};

/** One scene background per mood. */
export const BACKGROUNDS: Record<Mood, string> = {
  neutral: bg2,
  happy: bg1,
  thinking: bg3,
  worried: bg4,
  excited: bg5,
};
