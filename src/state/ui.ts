import { create } from "zustand";

export type Theme =
  | "light"
  | "dark"
  | "system"
  | "nature"
  | "darkblue"
  | "calmgreen"
  | "pastelpink"
  | "punkprincess";

export interface Toast {
  id: number;
  kind: "info" | "error" | "ok";
  text: string;
}

interface UiState {
  theme: Theme;
  settingsOpen: boolean;
  /** Auto-atualização em minutos (0 = desligado). */
  autoRefreshMin: number;
  toasts: Toast[];

  setTheme: (t: Theme) => void;
  setSettingsOpen: (v: boolean) => void;
  setAutoRefreshMin: (v: number) => void;
  pushToast: (kind: Toast["kind"], text: string) => void;
  dismissToast: (id: number) => void;
}

const THEME_KEY = "localfeed.theme";
const AUTOREFRESH_KEY = "localfeed.autoRefreshMin";

export const THEMES: Theme[] = [
  "system",
  "light",
  "dark",
  "nature",
  "darkblue",
  "calmgreen",
  "pastelpink",
  "punkprincess",
];

function loadTheme(): Theme {
  const v = localStorage.getItem(THEME_KEY);
  return v && (THEMES as string[]).includes(v) ? (v as Theme) : "system";
}

/** Aplica o tema no <html data-theme> (resolvendo "system" pela mídia). */
export function applyTheme(theme: Theme) {
  const resolved =
    theme === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : theme;
  document.documentElement.dataset.theme = resolved;
}

let nextToast = 1;

function loadAutoRefresh(): number {
  const v = Number(localStorage.getItem(AUTOREFRESH_KEY));
  return [0, 15, 30, 60].includes(v) ? v : 0;
}

export const useUi = create<UiState>((set) => ({
  theme: loadTheme(),
  settingsOpen: false,
  autoRefreshMin: loadAutoRefresh(),
  toasts: [],

  setTheme: (theme) => {
    localStorage.setItem(THEME_KEY, theme);
    applyTheme(theme);
    set({ theme });
  },
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setAutoRefreshMin: (autoRefreshMin) => {
    localStorage.setItem(AUTOREFRESH_KEY, String(autoRefreshMin));
    set({ autoRefreshMin });
  },
  pushToast: (kind, text) =>
    set((s) => ({ toasts: [...s.toasts, { id: nextToast++, kind, text }] })),
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));
