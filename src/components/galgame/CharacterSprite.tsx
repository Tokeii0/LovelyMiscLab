import { SPRITES } from "@/assets/galgame";
import type { Mood } from "@/store/galgame";

/** Stable pseudo-random index from a string key (FNV-1a). */
function pickIndex(key: string, n: number) {
  let h = 2166136261;
  for (let i = 0; i < key.length; i++) {
    h ^= key.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h) % n;
}

/** Misca — the CTF solving partner. Two art variants per mood; a per-turn `seed`
 * picks one (adds variety). Art comes from `@/assets/galgame`. */
export function CharacterSprite({ mood, seed = 0 }: { mood: Mood; seed?: number }) {
  const variants = SPRITES[mood];
  const src = variants[pickIndex(`${mood}:${seed}`, variants.length)];
  return (
    // No `key`: swapping the variant keeps the element, so the float animation
    // doesn't restart mid-bob.
    <img
      src={src}
      alt={`Misca (${mood})`}
      className="galgame-float h-full max-h-[88vh] w-auto max-w-full object-contain drop-shadow-[0_8px_40px_rgba(59,130,246,0.35)]"
    />
  );
}
