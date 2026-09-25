import { create } from "zustand";

/** Per-user UI preferences, kept in localStorage (never sent to the backend). */
export interface Prefs {
  /** Show every wire's type · value label, or only on hover / selection. */
  edgeLabels: "always" | "focus";
  /** Show the experimental 故事模式 (galgame) entry. */
  experimentalGalgame: boolean;
}

const KEY = "misclab-prefs-v1";
const DEFAULTS: Prefs = { edgeLabels: "focus", experimentalGalgame: false };

function load(): Prefs {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? { ...DEFAULTS, ...(JSON.parse(raw) as Partial<Prefs>) } : DEFAULTS;
  } catch {
    return DEFAULTS;
  }
}

interface PrefsState extends Prefs {
  set: <K extends keyof Prefs>(key: K, value: Prefs[K]) => void;
}

export const usePrefs = create<PrefsState>((set, get) => ({
  ...load(),
  set: (key, value) => {
    set({ [key]: value } as Partial<PrefsState>);
    try {
      const { edgeLabels, experimentalGalgame } = get();
      localStorage.setItem(KEY, JSON.stringify({ edgeLabels, experimentalGalgame }));
    } catch {
      /* ignore storage failures */
    }
  },
}));
