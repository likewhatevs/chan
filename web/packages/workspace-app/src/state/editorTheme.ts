// Apply the active editor theme as a `data-editor-theme` attribute
// on document.documentElement. The editor CSS (themes/*.css) keys
// every typography + chrome rule off that attribute crossed with
// the existing `data-theme` (light/dark) and `data-editor-density`
// (tight/standard) attributes.

import type { EditorTheme } from "../api/types";

export const DEFAULT_EDITOR_THEME: EditorTheme = "github";

export function applyEditorTheme(theme: EditorTheme): void {
  document.documentElement.setAttribute("data-editor-theme", theme);
}
