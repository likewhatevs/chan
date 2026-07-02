// Windows a self-managing launcher opened but no longer holds a live handle to.
//
// The client-side window manager keys `window.open` handles by `window_id`. A
// record that is present in the watch feed while this launcher holds no open
// handle for it is "orphaned": the launcher reloaded (its in-memory handles are
// gone) or a peer surface minted it. Those rows slow-flash so the user can
// click to (re)open the window in-app; the flash clears the moment a handle is
// (re)adopted.
//
// Keyed directly by `window_id` (the feed's reconciliation key). controlAttention
// is the CSS/UX model, not a drop-in: it is library_id-keyed and control-only.

interface WindowAttentionState {
  // window_ids awaiting a (re)open. A plain object so it is deeply reactive
  // under $state (a bare Set would need svelte/reactivity).
  windows: Record<string, true>;
}

export const windowAttention = $state<WindowAttentionState>({ windows: {} });

/** Flag a window as orphaned (in the feed, no live handle here) so its row
 * flashes for a (re)open click. */
export function markWindowAttention(windowId: string): void {
  windowAttention.windows[windowId] = true;
}

/** Clear a window's flash (a handle was adopted, or the record left the feed). */
export function clearWindowAttention(windowId: string): void {
  if (windowId in windowAttention.windows) delete windowAttention.windows[windowId];
}

/** Whether this window's row is awaiting a (re)open click. */
export function hasWindowAttention(windowId: string): boolean {
  return windowAttention.windows[windowId] === true;
}

/** Clear every flag (test reset; also a hard reset if ever needed). */
export function clearAllWindowAttention(): void {
  windowAttention.windows = {};
}
