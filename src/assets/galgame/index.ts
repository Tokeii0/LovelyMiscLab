// Story-mode art lives in `public/galgame/` and is served as plain files, so it
// loads only when 故事模式 is opened instead of being inlined as ~2 MB of base64
// into the single-file bundle that every launch has to parse.
import type { Mood } from "@/store/galgame";

const art = (name: string) => `/galgame/${name}.webp`;

/** Two art variants per mood (a per-turn seed picks one). */
export const SPRITES: Record<Mood, string[]> = {
  neutral: [art("misca-neutral"), art("misca-neutral-2")],
  happy: [art("misca-happy"), art("misca-happy-2")],
  thinking: [art("misca-thinking"), art("misca-thinking-2")],
  worried: [art("misca-worried"), art("misca-worried-2")],
  excited: [art("misca-excited"), art("misca-excited-2")],
};

/** One scene background per mood. */
export const BACKGROUNDS: Record<Mood, string> = {
  neutral: art("bg-2"),
  happy: art("bg-1"),
  thinking: art("bg-3"),
  worried: art("bg-4"),
  excited: art("bg-5"),
};
