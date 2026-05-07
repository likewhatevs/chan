// Per-context page width memory.
//
// Caps the centered editor content (.ProseMirror / .cm-content) on
// wide screens so a fullscreen window doesn't stretch lines the
// full viewport. The right cap depends on the window's current
// size, so we remember a value per screen + window bucket via a
// small LRU in localStorage. On boot, on debounced resize across a
// bucket boundary, and on cross-window updates we look up the
// remembered cap for the current key and apply it as a CSS var.
// Unknown keys default to `null` (unbounded) and don't write to the
// LRU until the user adjusts, so unused contexts stay out of the
// cache.
//
// localStorage is already per-origin per-browser-per-machine, so
// no device fingerprint is needed. Sync across machines is out of
// scope; sync across windows on the same browser rides the
// `storage` event for the current key.
//
// Stored value semantics:
//   number  -> px cap applied to the inner editor content
//   null    -> no cap (current default, content fills the
//              container minus its padding)

const STORAGE_KEY = "chan.pageWidth.lru";
const CSS_VAR = "--chan-page-max-width";
const LRU_CAP = 16;

export const PAGE_WIDTH_MIN = 480;
export const PAGE_WIDTH_MAX = 2000;
export const PAGE_WIDTH_STEP = 20;

/// Inner-window bucket size. Resizing the window without crossing
/// a bucket boundary keeps the same LRU key, so a slight drag
/// doesn't churn the cache or trigger a snap. 200px is wide enough
/// to collapse minor jiggle while still distinguishing fullscreen
/// from a half-screen window on the same monitor.
const WINDOW_BUCKET_PX = 200;

type LruValue = number | null;
type LruEntry = { key: string; value: LruValue; ts: number };
type LruState = { entries: LruEntry[] };

export const pageWidth = $state<{ value: LruValue; key: string | null }>({
  value: null,
  key: null,
});

function bucket(n: number): number {
  return Math.round(n / WINDOW_BUCKET_PX) * WINDOW_BUCKET_PX;
}

function currentScreenKey(): string {
  const sw = screen.width;
  const sh = screen.height;
  const dpr = Math.round((window.devicePixelRatio || 1) * 100) / 100;
  const iw = bucket(window.innerWidth);
  const ih = bucket(window.innerHeight);
  return `${sw}x${sh}@${dpr}|${iw}x${ih}`;
}

function clamp(v: number): number {
  return Math.max(PAGE_WIDTH_MIN, Math.min(PAGE_WIDTH_MAX, Math.round(v)));
}

function isValidEntry(x: unknown): x is LruEntry {
  if (!x || typeof x !== "object") return false;
  const e = x as Record<string, unknown>;
  if (typeof e.key !== "string" || typeof e.ts !== "number") return false;
  return e.value === null || typeof e.value === "number";
}

function readLru(): LruState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { entries: [] };
    const parsed = JSON.parse(raw) as { entries?: unknown };
    if (!parsed || !Array.isArray(parsed.entries)) return { entries: [] };
    return { entries: parsed.entries.filter(isValidEntry) };
  } catch {
    return { entries: [] };
  }
}

function writeLru(s: LruState): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
}

/// `undefined` = no entry, distinct from a stored null (which is a
/// remembered unbounded cap; not the same thing as "never set").
function lruLookup(s: LruState, key: string): LruValue | undefined {
  const hit = s.entries.find((e) => e.key === key);
  return hit ? hit.value : undefined;
}

function lruTouch(s: LruState, key: string, v: LruValue): LruState {
  const now = Date.now();
  const without = s.entries.filter((e) => e.key !== key);
  without.push({ key, value: v, ts: now });
  without.sort((a, b) => b.ts - a.ts);
  return { entries: without.slice(0, LRU_CAP) };
}

function applyToDom(v: LruValue): void {
  const root = document.documentElement;
  if (v == null) {
    root.style.removeProperty(CSS_VAR);
  } else {
    root.style.setProperty(CSS_VAR, `${v}px`);
  }
}

/// First-paint apply. Runs synchronously before any editor mounts
/// so the initial render is already capped to the remembered value
/// for this screen + window bucket.
export function applyInitialPageWidth(): void {
  if (typeof window === "undefined") return;
  pageWidth.key = currentScreenKey();
  const stored = lruLookup(readLru(), pageWidth.key);
  const v = stored === undefined ? null : stored;
  pageWidth.value = v;
  applyToDom(v);
}

/// User-driven update. Pass `null` to drop the cap entirely. Both
/// values (number and null) are written to the LRU so a deliberate
/// "full" setting for this context survives a relaunch.
export function setPageWidth(v: number | null): void {
  const next = v == null ? null : clamp(v);
  pageWidth.value = next;
  applyToDom(next);
  if (!pageWidth.key) return;
  writeLru(lruTouch(readLru(), pageWidth.key, next));
}

/// Watch for context changes that should swap the active cap:
///   - window resize crossing a bucket boundary (debounced)
///   - another window writing to the LRU on the same key
export function watchPageWidth(): () => void {
  if (typeof window === "undefined") return () => {};

  let resizeTimer: ReturnType<typeof setTimeout> | null = null;
  const onResize = () => {
    if (resizeTimer) clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
      resizeTimer = null;
      const next = currentScreenKey();
      if (next === pageWidth.key) return;
      pageWidth.key = next;
      const stored = lruLookup(readLru(), next);
      const v = stored === undefined ? null : stored;
      pageWidth.value = v;
      applyToDom(v);
    }, 200);
  };
  window.addEventListener("resize", onResize);

  const onStorage = (e: StorageEvent) => {
    if (e.key !== STORAGE_KEY || !pageWidth.key) return;
    const stored = lruLookup(readLru(), pageWidth.key);
    if (stored === undefined) return;
    if (stored === pageWidth.value) return;
    pageWidth.value = stored;
    applyToDom(stored);
  };
  window.addEventListener("storage", onStorage);

  return () => {
    window.removeEventListener("resize", onResize);
    window.removeEventListener("storage", onStorage);
    if (resizeTimer) clearTimeout(resizeTimer);
  };
}
