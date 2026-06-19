// Dark/light theme, applied as `data-theme` on the document element (the CSS
// tokens key off it) and persisted so a reload keeps the user's choice.

export type Theme = "dark" | "light";

const STORAGE_KEY = "chan-launcher-theme";

function prefersLight(): boolean {
  try {
    return typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: light)").matches;
  } catch {
    return false;
  }
}

function initialTheme(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "dark" || stored === "light") return stored;
  } catch {
    // Storage unavailable (private mode): fall through to the system default.
  }
  return prefersLight() ? "light" : "dark";
}

export const themeState = $state<{ theme: Theme }>({ theme: initialTheme() });

export function applyTheme(): void {
  document.documentElement.setAttribute("data-theme", themeState.theme);
}

export function toggleTheme(): void {
  themeState.theme = themeState.theme === "dark" ? "light" : "dark";
  try {
    localStorage.setItem(STORAGE_KEY, themeState.theme);
  } catch {
    // Best-effort persistence; the in-memory toggle still works without it.
  }
  applyTheme();
}
