// The launcher's client-side window manager: it owns the window.open handles for
// browser-minted workspace/terminal windows and reconciles them against the
// library watch feed.
//
// On a self-managed (devserver/PWA) surface the launcher is not a desktop bridge,
// so it opens windows as same-origin browser windows instead of driving native
// ones. It keys each handle by window_id (the feed's reconciliation key), mints
// via the widened createWindow with origin:"browser" (so the desktop watcher
// never grows a native twin), and on the feed's absence-only discard closes the
// matching handle. A reload wipes the in-memory handle map, so the reconciler
// re-flags visible browser-origin records it holds no handle for (via
// windowAttention) so their rows flash for a re-open click.
//
// Inert under demoState.enabled: a marketing embed never spawns windows.

import { backend } from "../api/backend";
import type { WindowKind, WindowRecord, WindowSet } from "../api/library";
import { windowUrl } from "../lib/windowUrl";
import { demoState } from "./demo.svelte";
import { clearWindowAttention, markWindowAttention } from "./windowAttention.svelte";

// window_id -> the open browser window. Imperative (the reactive surface is
// windowAttention); a reload wipes it and the reconciler re-flags orphans.
const handles = new Map<string, Window>();
// window_ids from the last feed push, so the reconciler detects removals (the
// feed signals a discard by ABSENCE, never a tombstone).
let prevIds = new Set<string>();

function servingOrigin(): string {
  return typeof location === "undefined" ? "" : location.origin;
}

// The live handle for a window, dropping a stale one the user closed by hand.
function liveHandle(id: string): Window | null {
  const h = handles.get(id);
  if (h && !h.closed) return h;
  if (h) handles.delete(id);
  return null;
}

/** Mint a browser window of the local library and open it in-app. Call this
 * DIRECTLY from a user gesture: it opens the blank window synchronously, before
 * the mint await, so the browser does not treat the later navigation as a popup.
 * On mint failure (e.g. the workspace is not running -> 409) the blank window is
 * closed and the error rethrown for the caller's banner. `actingWindowId` claims
 * the leader identity for the per-tenant mint gate. */
export async function mintWindow(
  kind: WindowKind,
  opts: { workspacePath?: string; actingWindowId?: string } = {},
): Promise<WindowRecord | null> {
  if (demoState.enabled) return null;
  const blank = servingOrigin() ? window.open("", "_blank") : null;
  try {
    const rec = await backend.createWindow(kind, {
      workspacePath: opts.workspacePath,
      origin: "browser",
      actingWindowId: opts.actingWindowId,
    });
    if (blank) {
      blank.location.href = windowUrl(rec, servingOrigin());
      handles.set(rec.window_id, blank);
    }
    // A blocked popup leaves no handle; the reconciler flags rec once it lands in
    // the feed so its row flashes for a click-to-open.
    clearWindowAttention(rec.window_id);
    return rec;
  } catch (e) {
    blank?.close();
    throw e;
  }
}

/** Open (or re-focus) an existing record's window in-app. The window is named by
 * window_id so a second click focuses the same same-origin window instead of
 * opening a duplicate. Used by the follower open-click and orphan re-open. */
export function openWindowRecord(record: WindowRecord): Window | null {
  if (demoState.enabled || !servingOrigin()) return null;
  const h = window.open(windowUrl(record, servingOrigin()), record.window_id);
  if (h) {
    handles.set(record.window_id, h);
    clearWindowAttention(record.window_id);
    h.focus?.();
  }
  return h;
}

/** Leader-side close/hide of a record from the launcher: run the bridgeless web
 * op (discard, or visibility=hidden) and close this launcher's local handle.
 * `actingWindowId` claims the leader identity for the per-tenant gate. */
export async function closeWindowRecord(
  record: WindowRecord,
  opts: { hide?: boolean; actingWindowId?: string } = {},
): Promise<void> {
  if (demoState.enabled) return;
  if (opts.hide) await backend.setWindowVisibility(record.window_id, true, opts.actingWindowId);
  else await backend.discardWindow(record.window_id, opts.actingWindowId);
  handles.get(record.window_id)?.close();
  handles.delete(record.window_id);
  clearWindowAttention(record.window_id);
}

/** Flip a window's server-persisted visibility from a self-managed launcher (the
 * bridgeless Eye toggle): hide a visible window, un-hide a hidden one, keyed on
 * the feed's `hidden`. `actingWindowId` claims the leader identity for the
 * per-tenant gate (the server 403s a mismatching claim). This touches only the
 * shared visibility state; the local browser handle is the OPEN button's job, so
 * nothing here opens or closes it. */
export async function toggleWindowVisibility(
  record: WindowRecord,
  actingWindowId?: string,
): Promise<void> {
  if (demoState.enabled) return;
  await backend.setWindowVisibility(record.window_id, !(record.hidden ?? false), actingWindowId);
}

/** Reconcile the handle map against a feed snapshot. Closes handles whose record
 * left the feed (absence == discard), and flags a VISIBLE browser-origin record
 * this launcher holds no live handle for as an orphan (a reload lost the handle,
 * or a peer surface minted it) so its row flashes for a re-open click. A hidden
 * or native record is never flagged. */
export function reconcileWindows(set: WindowSet): void {
  if (demoState.enabled) return;
  const currentIds = new Set(set.windows.map((w) => w.window_id));
  for (const id of prevIds) {
    if (!currentIds.has(id)) {
      handles.get(id)?.close();
      handles.delete(id);
      clearWindowAttention(id);
    }
  }
  for (const w of set.windows) {
    if (liveHandle(w.window_id)) {
      clearWindowAttention(w.window_id);
    } else if (w.origin === "browser" && !w.hidden) {
      markWindowAttention(w.window_id);
    } else {
      clearWindowAttention(w.window_id);
    }
  }
  prevIds = currentIds;
}

/** Whether this launcher holds a live handle for a window (its row is "open"
 * here, not an orphan). */
export function hasWindowHandle(id: string): boolean {
  return liveHandle(id) !== null;
}

/** Test/reset hook: drop all handles and the diff snapshot. */
export function resetWindowManager(): void {
  handles.clear();
  prevIds = new Set();
}
