/// Per-window pane-highlight colour, carried on the window URL.
///
/// Each library has a pane-highlight colour. The desktop appends it to the
/// window URL at mint time as `?pane=<hex>` (e.g. `?pane=%23e58c4d`). On boot
/// the SPA reads that param and, if it is a valid hex colour, sets the
/// `--pane-highlight-color` CSS variable on the document root. The active-pane
/// highlight (`.pane.focused` border + focus halo in `Pane.svelte`) prefers
/// that variable, falling back to the `data-focus-color` presets when it is
/// absent. Only a validated, normalized hex is ever written into the style, so
/// the URL can never inject arbitrary CSS.

import type { FocusColor } from "./tabs.svelte";

const CSS_VAR = "--pane-highlight-color";

/// The four named per-window focus-colour presets mapped to the exact border
/// hexes the active-pane highlight uses. `blue` is the `--pane-focus` literal
/// from App.svelte's dark theme; orange/green/pink mirror the `.pane`
/// `data-focus-color` preset rules in Pane.svelte. This is the single source
/// the colour menu writes to the per-library store and the boot-seed reads
/// back from the `?pane=` hex, so the two stay in lockstep.
export const NAMED_PANE_HEX: Record<FocusColor, string> = {
  blue: "#388bfd",
  orange: "#f97316",
  green: "#22c55e",
  pink: "#ff5fb7",
};

/// Reverse of `NAMED_PANE_HEX`: given a hex (already normalized to canonical
/// `#rrggbb`), return the matching preset name, or null when it is not one of
/// the four presets. Used by the boot-seed to set the menu checkmark to match
/// the library colour the window opened with; a custom (non-preset) colour
/// leaves the checkmark unset, which is correct -- no preset is "selected".
export function namedForPaneHex(hex: string | null | undefined): FocusColor | null {
  const norm = normalizeHexColor(hex);
  if (!norm) return null;
  for (const [name, value] of Object.entries(NAMED_PANE_HEX)) {
    if (value === norm) return name as FocusColor;
  }
  return null;
}

/// Validate + normalize a hex colour. Accepts `#rgb`, `#rrggbb`, `rgb`,
/// `rrggbb` (the leading `#` is optional) and returns the canonical
/// `#rrggbb` form. Anything else (named colours, `javascript:` URLs, bad
/// lengths, non-hex digits) returns null.
export function normalizeHexColor(raw: string | null | undefined): string | null {
  if (typeof raw !== "string") return null;
  const trimmed = raw.trim();
  const body = trimmed.startsWith("#") ? trimmed.slice(1) : trimmed;
  if (!/^[0-9a-fA-F]{3}$|^[0-9a-fA-F]{6}$/.test(body)) return null;
  const full =
    body.length === 3
      ? body
          .split("")
          .map((c) => c + c)
          .join("")
      : body;
  return `#${full.toLowerCase()}`;
}

/// Read the `?pane=` query param. Returns the raw (URL-decoded) value, or null
/// in non-browser (test) contexts or when the param is absent.
function readPaneParam(): string | null {
  try {
    return new URLSearchParams(location.search).get("pane");
  } catch {
    return null;
  }
}

/// First-paint apply. Reads `?pane=` from the window URL and, when it is a
/// valid hex colour, sets `--pane-highlight-color` on the document root.
/// Absent or invalid input is a no-op, leaving the variable unset so the
/// existing `data-focus-color` presets/defaults stay in effect.
export function applyInitialPaneColor(): void {
  if (typeof document === "undefined") return;
  const hex = normalizeHexColor(readPaneParam());
  if (hex) document.documentElement.style.setProperty(CSS_VAR, hex);
}

/// Live-apply a colour pushed by the per-library focus-colour watch,
/// `GET /api/library/local-color/watch`). A valid hex sets
/// `--pane-highlight-color` on the document root, so every pane of this window
/// recolours the instant any window of the library changes the colour.
///
/// A null / absent / invalid colour is treated as "no override": the current
/// value is LEFT IN PLACE, never cleared. The watch pushes the current colour on
/// connect, so a library with no persisted colour pushes `{ color: null }` right
/// after boot -- clearing the var there would clobber the `?pane=` boot seed back
/// to the default accent (Bug A). Same validation as the `?pane=` path, so a
/// watch frame can never inject arbitrary CSS.
export function applyLivePaneColor(color: string | null): void {
  if (typeof document === "undefined") return;
  const hex = normalizeHexColor(color);
  if (hex) document.documentElement.style.setProperty(CSS_VAR, hex);
}

/// Boot-seed the per-window focus-colour menu checkmark so it matches the
/// library colour the window opened with. If `?pane=` resolved to one of the
/// four preset hexes, invoke `setColor` with that named preset so
/// `focusColorForWindow()` (the menu checkmark) agrees with the colour the
/// active pane is actually showing. A non-preset (custom) hex, an absent
/// param, or a non-browser/test context is a no-op -- the menu keeps its
/// default and no preset reads as "selected". Takes the setter as a callback
/// to stay free of the tabs state module (no import cycle, trivially
/// testable).
export function seedInitialFocusColor(
  setColor: (color: FocusColor) => void,
): void {
  if (typeof location === "undefined") return;
  const named = namedForPaneHex(readPaneParam());
  if (named) setColor(named);
}

/// Sync the per-window focus-colour MENU to a colour pushed by the live watch.
/// `applyLivePaneColor` recolours the active border (the doc-root var, which
/// overrides `data-focus-color`), but the menu checkmark + any NEW split pane's
/// `data-focus-color` read `focusColorForWindow()` (`layout.focusColor`) -- left
/// stale, they disagree with the border the window is actually showing. So when
/// a pushed colour maps to one of the four presets, select it; a custom (non-
/// preset), null, or invalid colour leaves the menu as-is (no preset reads as
/// "selected"). The live counterpart of `seedInitialFocusColor` (which seeds
/// from `?pane=`); takes the setter as a callback to stay free of the tabs state
/// module (no import cycle, trivially testable).
export function syncLiveFocusColorMenu(
  color: string | null,
  setColor: (color: FocusColor) => void,
): void {
  const named = namedForPaneHex(color);
  if (named) setColor(named);
}
