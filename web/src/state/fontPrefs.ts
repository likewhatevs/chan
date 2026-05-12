// Apply per-drive font preferences as CSS custom properties on
// document.documentElement. The Wysiwyg + Source editors and the
// markdown-rendered content read these vars directly, so changing
// them re-renders without rebuilding any component.
//
// Variable naming mirrors the Rust enum names so the source of
// truth matches: --chan-font-heading1-family, --chan-font-heading1-size, …

import type { FontPrefs } from "../api/types";

const ROLES = ["heading1", "heading2", "heading3", "normal", "code", "quote"] as const;

export function applyFontPrefs(prefs: FontPrefs): void {
  const root = document.documentElement;
  for (const role of ROLES) {
    const spec = prefs[role];
    if (!spec) continue;
    root.style.setProperty(`--chan-font-${role}-family`, spec.family);
    root.style.setProperty(`--chan-font-${role}-size`, `${spec.size}px`);
  }
}

/// Defaults baked into the editor CSS. Used as the fallback when no
/// drive preferences have loaded yet so the UI doesn't render
/// with empty CSS variables.
export const DEFAULT_FONT_PREFS: FontPrefs = {
  heading1: {
    family:
      "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif",
    size: 32,
  },
  heading2: {
    family:
      "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif",
    size: 24,
  },
  heading3: {
    family:
      "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif",
    size: 20,
  },
  normal: {
    family:
      "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif",
    size: 16,
  },
  code: {
    family: "ui-monospace, SFMono-Regular, monospace",
    size: 14,
  },
  quote: {
    family:
      "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif",
    size: 16,
  },
};
