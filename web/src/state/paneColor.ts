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

const CSS_VAR = "--pane-highlight-color";

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
