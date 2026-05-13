// Page-width cap as a ratio of the current window width.
//
// Caps the centered editor content (.ProseMirror / .cm-content) on
// wide screens so a fullscreen window doesn't stretch lines the
// full viewport. The cap is stored as a single ratio in (0, 1]:
//   1.0  -> no cap (content fills the container minus padding)
//   r<1  -> cap = round(window.innerWidth * r)
//
// Storing a ratio (not absolute pixels) is what makes the cap
// "relative" in the user-facing sense:
//   - resizing the browser window: the cap follows innerWidth, so
//     the editor stays at the same % of the window
//   - browser zoom (cmd +/-): zoom rescales CSS pixels, which
//     changes innerWidth in CSS units; the cap rescales with it
//     instead of getting wedged at a now-meaningless pixel value
//
// localStorage is per-origin per-browser-per-machine; sync across
// machines is out of scope, sync across windows on the same browser
// rides the `storage` event.

const STORAGE_KEY = "chan.pageWidth.ratio";
const CSS_VAR = "--chan-page-max-width";

/// Independent cap for the assistant overlay's prompt column. Lives
/// next to `pageWidth` because it shares the slider bounds + ratio
/// idiom, but writes to its own localStorage key and a separate CSS
/// variable so adjusting one doesn't move the other. The assistant
/// menu's slider feeds this state, and the prompt-wrap consumes the
/// resolved percentage as a `max-width` (% of the overlay column, so
/// no resize bookkeeping is needed).
const ASSISTANT_STORAGE_KEY = "chan.assistantPromptWidth.ratio";

/// Slider bounds in percent. 100 % is the "no cap" sentinel; below
/// 25 % the editor would be unusably narrow on any normal window,
/// so we clamp there.
export const PAGE_WIDTH_MIN_PCT = 25;
export const PAGE_WIDTH_MAX_PCT = 100;
export const PAGE_WIDTH_STEP_PCT = 5;

/// Floor on the resolved pixel cap. Even at the smallest ratio on a
/// tiny window we don't want the cap to collapse to a sliver.
const MIN_RESOLVED_PX = 240;

export const pageWidth = $state<{ ratio: number }>({ ratio: 1 });

export const assistantPromptWidth = $state<{ ratio: number }>({ ratio: 1 });

/// Global overlay-maximize toggle. When on, every OverlayShell
/// widens its panel from `min(1200px, calc(100vw - 48px))` to
/// `calc(100vw - 88px)` so the side gap matches the top safe-area
/// + 44px chrome buffer. Lives next to the page-width state because
/// both knobs persist across reloads under the same module and the
/// menu items that toggle them sit in the same hamburger surfaces.
const OVERLAY_MAX_KEY = "chan.overlayMaximized";
export const overlayMaximized = $state<{ on: boolean }>({ on: false });

function clampRatio(r: number): number {
  if (!Number.isFinite(r)) return 1;
  const lo = PAGE_WIDTH_MIN_PCT / 100;
  const hi = PAGE_WIDTH_MAX_PCT / 100;
  return Math.max(lo, Math.min(hi, r));
}

function readRatio(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return 1;
    const n = Number(raw);
    if (!Number.isFinite(n)) return 1;
    return clampRatio(n);
  } catch {
    return 1;
  }
}

function writeRatio(r: number): void {
  try {
    localStorage.setItem(STORAGE_KEY, String(r));
  } catch {
    /* quota or disabled storage; the in-memory value still applies */
  }
}

function applyToDom(r: number): void {
  const root = document.documentElement;
  if (r >= 1) {
    root.style.removeProperty(CSS_VAR);
    root.classList.remove("chan-page-capped");
    return;
  }
  const px = Math.max(MIN_RESOLVED_PX, Math.round(window.innerWidth * r));
  root.style.setProperty(CSS_VAR, `${px}px`);
  root.classList.add("chan-page-capped");
}

/// First-paint apply. Runs synchronously before any editor mounts
/// so the initial render is already capped to the remembered ratio.
export function applyInitialPageWidth(): void {
  if (typeof window === "undefined") return;
  pageWidth.ratio = readRatio();
  applyToDom(pageWidth.ratio);
  assistantPromptWidth.ratio = readAssistantRatio();
  overlayMaximized.on = readOverlayMaximized();
}

function readAssistantRatio(): number {
  try {
    const raw = localStorage.getItem(ASSISTANT_STORAGE_KEY);
    if (!raw) return 1;
    const n = Number(raw);
    if (!Number.isFinite(n)) return 1;
    return clampRatio(n);
  } catch {
    return 1;
  }
}

/// User-driven update for the assistant prompt cap. Pure state +
/// persistence; the prompt-wrap reads `assistantPromptWidth.ratio`
/// directly as a `max-width` percentage, so no DOM apply is needed.
export function setAssistantPromptWidth(r: number): void {
  const next = clampRatio(r);
  assistantPromptWidth.ratio = next;
  try {
    localStorage.setItem(ASSISTANT_STORAGE_KEY, String(next));
  } catch {
    /* quota or disabled storage; in-memory value still applies */
  }
}

function readOverlayMaximized(): boolean {
  try {
    return localStorage.getItem(OVERLAY_MAX_KEY) === "1";
  } catch {
    return false;
  }
}

/// Panel width an OverlayShell resolves to for a given maximize
/// state. Mirrors the CSS `width` in OverlayShell.svelte exactly,
/// kept in sync by hand: normal mode caps at 1200px (with a 24px
/// gutter on each side); maximized mode trims a symmetric 44px so
/// the side gap visually matches the top safe-area + chrome buffer.
function panelWidthFor(maxed: boolean): number {
  if (typeof window === "undefined") return 0;
  const vw = window.innerWidth;
  return maxed ? vw - 88 : Math.min(1200, vw - 48);
}

export function setOverlayMaximized(on: boolean): void {
  if (overlayMaximized.on === on) return;
  // Recalibrate the assistant prompt-width ratio so the rendered
  // prompt-wrap retains its absolute pixel width across the
  // maximize toggle. Without this, a 70% cap on a 1200px panel
  // (840px wrap) would jump to 70% of a much wider maximized
  // panel and visually balloon. The clamp inside
  // setAssistantPromptWidth keeps the new ratio in [0.25, 1].
  const before = panelWidthFor(overlayMaximized.on);
  const after = panelWidthFor(on);
  overlayMaximized.on = on;
  try {
    localStorage.setItem(OVERLAY_MAX_KEY, on ? "1" : "0");
  } catch {
    /* quota or disabled storage; in-memory value still applies */
  }
  if (before > 0 && after > 0 && before !== after) {
    const targetPx = assistantPromptWidth.ratio * before;
    setAssistantPromptWidth(targetPx / after);
  }
}

/// User-driven update. Pass a ratio in (0, 1]; 1 means unbounded.
export function setPageWidth(r: number): void {
  const next = clampRatio(r);
  pageWidth.ratio = next;
  applyToDom(next);
  writeRatio(next);
}

/// Per-element apply. Each Pane.svelte instance subscribes to its
/// own .editor-wrap width via ResizeObserver and calls this; the
/// resulting cap is pane-relative instead of window-relative, so
/// splitting one pane into two halves correctly halves the cap.
/// Window resize / browser zoom also flow through the same
/// observer because the pane shrinks with the window.
export function applyPageWidthToElement(
  el: HTMLElement,
  containerWidth: number,
  r: number,
): void {
  if (r >= 1 || containerWidth <= 0) {
    el.style.removeProperty(CSS_VAR);
    return;
  }
  const px = Math.max(MIN_RESOLVED_PX, Math.round(containerWidth * r));
  el.style.setProperty(CSS_VAR, `${px}px`);
}

/// Re-apply on viewport changes (resize, browser zoom) and pick up
/// updates from other windows on the same origin via the `storage`
/// event.
export function watchPageWidth(): () => void {
  if (typeof window === "undefined") return () => {};

  let raf: number | null = null;
  const reapply = () => {
    raf = null;
    applyToDom(pageWidth.ratio);
  };
  const onResize = () => {
    if (raf != null) return;
    raf = requestAnimationFrame(reapply);
  };
  window.addEventListener("resize", onResize);

  const onStorage = (e: StorageEvent) => {
    if (e.key !== STORAGE_KEY) return;
    const next = readRatio();
    if (next === pageWidth.ratio) return;
    pageWidth.ratio = next;
    applyToDom(next);
  };
  window.addEventListener("storage", onStorage);

  return () => {
    window.removeEventListener("resize", onResize);
    window.removeEventListener("storage", onStorage);
    if (raf != null) cancelAnimationFrame(raf);
  };
}
