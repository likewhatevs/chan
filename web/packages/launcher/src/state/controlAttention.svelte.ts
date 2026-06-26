// Devserver control terminals awaiting attention.
//
// When a connected devserver's CONTROL terminal's inner process exits, the
// desktop keeps the control window row (it no longer reaps it on PTY exit) and
// emits `devserver-control-closed`. The launcher flashes that row's eye yellow in
// the Open-windows feed to request attention, an additive cue beside the
// re-run/edit/abandon survey modal. The flash clears when the user acts -- shows
// or focuses the window, or reconnects the devserver.
//
// Keyed by the devserver's LIBRARY id (the id the control window record carries)
// so the feed matches the flashing row directly. The `devserver-control-closed`
// event carries the devserver id, resolved to its library id at mark time.

import { library } from "./library.svelte";

interface ControlAttentionState {
  // Library ids whose control terminal needs attention. A plain object so it is
  // deeply reactive under $state (a bare Set would need svelte/reactivity).
  libs: Record<string, true>;
}

export const controlAttention = $state<ControlAttentionState>({ libs: {} });

/** Flag a devserver's control terminal for attention (its inner process exited).
 * Resolves the devserver id to the library id the feed keys on. A devserver with
 * a control terminal has connected at least once, so its library id is known; an
 * unknown id or one with no library id is a no-op. */
export function markControlAttention(devserverId: string): void {
  const ds = library.devservers.find((d) => d.id === devserverId);
  if (ds?.library_id) controlAttention.libs[ds.library_id] = true;
}

/** Clear a library's control-attention (the user showed/focused the window, or
 * the devserver reconnected). */
export function clearControlAttention(libraryId: string): void {
  if (libraryId in controlAttention.libs) delete controlAttention.libs[libraryId];
}

/** Whether the control row of this library is awaiting attention. */
export function hasControlAttention(libraryId: string): boolean {
  return controlAttention.libs[libraryId] === true;
}

/** Clear every attention flag (test reset; also a hard reset if ever needed). */
export function clearAllControlAttention(): void {
  controlAttention.libs = {};
}
