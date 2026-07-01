import { create } from "zustand";

export type Theme = "light" | "dark";

const KEY = "misclab-theme";

function readInitial(): Theme {
  try {
    return localStorage.getItem(KEY) === "dark" ? "dark" : "light";
  } catch {
    return "light";
  }
}

function apply(theme: Theme) {
  document.documentElement.classList.toggle("dark", theme === "dark");
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    /* ignore */
  }
}

interface ThemeState {
  theme: Theme;
  setTheme: (t: Theme) => void;
  toggle: () => void;
}

/** Light by default; toggles the `dark` class on <html> and persists the choice. */
export const useThemeStore = create<ThemeState>((set, get) => {
  // Sync the <html> class to the persisted/default theme once at startup so the
  // DOM (Tailwind `dark:` variants) and the store (React Flow colorMode) agree.
  const initial = readInitial();
  apply(initial);
  return {
    theme: initial,
    setTheme: (t) => {
      apply(t);
      set({ theme: t });
    },
    toggle: () => {
      const t: Theme = get().theme === "dark" ? "light" : "dark";
      apply(t);
      set({ theme: t });
    },
  };
});
