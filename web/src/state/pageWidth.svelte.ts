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
    return;
  }
  const px = Math.max(MIN_RESOLVED_PX, Math.round(window.innerWidth * r));
  root.style.setProperty(CSS_VAR, `${px}px`);
}

/// First-paint apply. Runs synchronously before any editor mounts
/// so the initial render is already capped to the remembered ratio.
export function applyInitialPageWidth(): void {
  if (typeof window === "undefined") return;
  pageWidth.ratio = readRatio();
  applyToDom(pageWidth.ratio);
}

/// User-driven update. Pass a ratio in (0, 1]; 1 means unbounded.
export function setPageWidth(r: number): void {
  const next = clampRatio(r);
  pageWidth.ratio = next;
  applyToDom(next);
  writeRatio(next);
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
