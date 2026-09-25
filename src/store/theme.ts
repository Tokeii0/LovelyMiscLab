import { create } from "zustand";

export type Theme = "light" | "dark";
/** What the user picked; "system" follows the OS light/dark setting. */
export type ThemePref = Theme | "system";

const KEY = "misclab-theme";
const media =
  typeof window !== "undefined" && window.matchMedia
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;

function readPref(): ThemePref {
  try {
    const v = localStorage.getItem(KEY);
    return v === "dark" || v === "light" ? v : "system";
  } catch {
    return "system";
  }
}

function resolve(pref: ThemePref): Theme {
  if (pref !== "system") return pref;
  return media?.matches ? "dark" : "light";
}

/** Sync <html>: the `dark` class (Tailwind variants) and `color-scheme`, so native
 * controls — selects, number spinners, scrollbars — match the theme too. */
function apply(theme: Theme) {
  document.documentElement.classList.toggle("dark", theme === "dark");
  document.documentElement.style.colorScheme = theme;
}

interface ThemeState {
  pref: ThemePref;
  /** The theme actually shown. */
  theme: Theme;
  setPref: (p: ThemePref) => void;
  /** Title-bar button: switch to the opposite of what's shown (an explicit choice). */
  toggle: () => void;
}

export const useThemeStore = create<ThemeState>((set, get) => {
  const pref = readPref();
  const initial = resolve(pref);
  apply(initial);
  media?.addEventListener("change", () => {
    if (get().pref !== "system") return;
    const theme = resolve("system");
    apply(theme);
    set({ theme });
  });
  const setPref = (p: ThemePref) => {
    const theme = resolve(p);
    apply(theme);
    try {
      localStorage.setItem(KEY, p);
    } catch {
      /* ignore */
    }
    set({ pref: p, theme });
  };
  return {
    pref,
    theme: initial,
    setPref,
    toggle: () => setPref(get().theme === "dark" ? "light" : "dark"),
  };
});
