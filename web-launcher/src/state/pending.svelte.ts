// In-flight feedback for the mutating registry actions (workspace on/off, served
// devserver-workspace on/off, devserver connect/disconnect). Clicking one of
// those gives no feedback today — the row flips silently and the op can take a
// while — so this overlays a spinner on the acting row's button until the op
// settles.
//
// A reactive map of "pending markers" keyed per item, each recording the desired
// end-state (`target`) + a start timestamp (`ts`). Markers persist to
// localStorage, so a spinner survives a launcher reload; on reload `loadLibrary`
// fetches the real registries and `reconcile()` clears any marker whose row has
// already reached its target (the op finished while we were away) — so the
// spinner picks up the latest real state instead of spinning forever. A timeout
// safety clears abandoned markers, after which the row just shows its real state.
//
// SPA-only, no backend change. localStorage is standard in the launcher's
// WKWebView; if it is ever unavailable the markers degrade to in-memory (the
// spinner still works in-session, it just won't survive a reload).

/** Desired end-state a marker waits for. Workspaces use on/off; devservers use
 * connected/disconnected. */
export type PendingTarget = "on" | "off" | "connected" | "disconnected";

interface PendingEntry {
  target: PendingTarget;
  /** Epoch ms when the op began — drives the timeout safety. */
  ts: number;
}

interface PendingState {
  /** Active markers keyed by `ws:{id}` / `served:{devserverId}:{prefix}` /
   * `ds:{id}`. A plain object — deeply reactive under $state. */
  markers: Record<string, PendingEntry>;
}

// Clear a marker once it is older than this: a failed/abandoned op then stops
// spinning and the row shows its real state. Evaluated on each reconcile + on
// every isPending read, so no leaked timer is needed (mirrors the library
// re-fetch coalescing, which is deliberately timer-free).
const PENDING_TIMEOUT_MS = 45_000;

const STORAGE_KEY = "chan-launcher-pending";

function isTarget(v: unknown): v is PendingTarget {
  return v === "on" || v === "off" || v === "connected" || v === "disconnected";
}

/** Read the persisted markers, dropping anything malformed. Best-effort: a
 * surface without localStorage (or a parse error) just starts empty. */
function load(): Record<string, PendingEntry> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const out: Record<string, PendingEntry> = {};
    for (const [k, v] of Object.entries(parsed)) {
      const e = v as { target?: unknown; ts?: unknown };
      if (e && isTarget(e.target) && typeof e.ts === "number") {
        out[k] = { target: e.target, ts: e.ts };
      }
    }
    return out;
  } catch {
    return {};
  }
}

function persist(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(pending.markers));
  } catch {
    // localStorage unavailable (or quota): in-memory only — the spinner still
    // works this session, it just won't survive a reload.
  }
}

// Hydrate from localStorage at module init so a reload shows the spinner
// immediately, before loadLibrary's reconcile lands.
export const pending = $state<PendingState>({ markers: load() });

/** Re-read the persisted markers (a reload re-inits the module and does this;
 * exposed for tests to exercise the load path deterministically). */
export function hydratePending(): void {
  pending.markers = load();
}

export const wsKey = (id: string): string => `ws:${id}`;
export const servedKey = (devserverId: string, prefix: string): string =>
  `served:${devserverId}:${prefix}`;
export const dsKey = (id: string): string => `ds:${id}`;

/** Whether the row keyed by `key` is mid-op (drives its spinner). Treats a
 * timed-out marker as not-pending even before reconcile deletes it, so a stale
 * marker never spins. */
export function isPending(key: string): boolean {
  const e = pending.markers[key];
  if (!e) return false;
  return Date.now() - e.ts <= PENDING_TIMEOUT_MS;
}

/** Mark an op in flight (on action click): record target + start time, persist,
 * and start the spinner. Overwrites any prior marker for the same row. */
export function beginPending(key: string, target: PendingTarget): void {
  pending.markers[key] = { target, ts: Date.now() };
  persist();
}

/** Clear a marker (on reject, or once reconciled): stop the spinner. */
export function clearPending(key: string): void {
  if (key in pending.markers) {
    delete pending.markers[key];
    persist();
  }
}

/** Clear every marker (test reset; also a hard reset if ever needed). */
export function clearAllPending(): void {
  pending.markers = {};
  persist();
}

/** Reconcile markers against the latest real state. `current` maps the same keys
 * to each row's effective state. A marker is cleared when its row already
 * reached the target, the row is gone, or the marker has timed out; otherwise it
 * keeps spinning. Called after every registry refresh/feed push and on
 * loadLibrary (mount/reload). */
export function reconcile(current: Record<string, PendingTarget>): void {
  const now = Date.now();
  let changed = false;
  for (const key of Object.keys(pending.markers)) {
    const entry = pending.markers[key]!;
    const cur = current[key];
    if (cur === undefined || cur === entry.target || now - entry.ts > PENDING_TIMEOUT_MS) {
      delete pending.markers[key];
      changed = true;
    }
  }
  if (changed) persist();
}
