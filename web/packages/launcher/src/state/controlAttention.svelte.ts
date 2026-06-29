// Devserver control terminals awaiting attention.
//
// When a connected devserver's CONTROL terminal's inner process exits, the
// desktop keeps the control window row (it no longer reaps it on PTY exit) and
// emits `devserver-control-closed`. The launcher flashes that row's eye yellow in
// the Open-windows feed to request attention. The flash clears when the user
// acts -- shows or focuses the window, or reconnects the devserver.
//
// Keyed by the devserver's LIBRARY id (the id the control window record carries)
// so the feed matches the flashing row directly. The `devserver-control-closed`
// event carries the devserver id, resolved to its library id at mark time.

import { library } from "./library.svelte";

const CONTROL_WINDOW_PREFIX = "control-terminal-";
const PENDING_ATTENTION_MS = 30_000;

interface ControlAttentionState {
  // Library ids whose control terminal needs attention. A plain object so it is
  // deeply reactive under $state (a bare Set would need svelte/reactivity).
  libs: Record<string, true>;
  // Devserver ids that emitted before the launcher had enough registry/window
  // state to resolve their library id.
  pendingDevservers: Record<string, number>;
}

export const controlAttention = $state<ControlAttentionState>({
  libs: {},
  pendingDevservers: {},
});

function controlLibraryId(devserverId: string): string | null {
  // The desktop emits as soon as the control script exits; on a first connect
  // that can beat the devserver registry refresh that fills `library_id`.
  // The control row is already in the window feed under its real library id.
  const byControlWindow = library.windows.find(
    (w) => w.control && w.window_id === `${CONTROL_WINDOW_PREFIX}${devserverId}`,
  );
  if (byControlWindow) return byControlWindow.library_id;

  const ds = library.devservers.find((d) => d.id === devserverId);
  if (ds?.library_id) return ds.library_id;

  const direct = library.windows.find((w) => w.control && w.library_id === devserverId);
  return direct?.library_id ?? null;
}

/** Flag a devserver's control terminal for attention (its inner process exited).
 * Resolves the devserver id to the library id the feed keys on, or queues it
 * briefly until the matching control-window row / devserver entry arrives. */
export function markControlAttention(devserverId: string): void {
  const libraryId = controlLibraryId(devserverId);
  if (libraryId) {
    controlAttention.libs[libraryId] = true;
    delete controlAttention.pendingDevservers[devserverId];
  } else {
    controlAttention.pendingDevservers[devserverId] = Date.now();
  }
}

/** Resolve any control-closed event that arrived before the launcher knew the
 * devserver's library id. Called from App's registry/window-feed reactive pass. */
export function resolvePendingControlAttention(): void {
  const now = Date.now();
  for (const [devserverId, ts] of Object.entries(controlAttention.pendingDevservers)) {
    if (now - ts > PENDING_ATTENTION_MS) {
      delete controlAttention.pendingDevservers[devserverId];
      continue;
    }
    const libraryId = controlLibraryId(devserverId);
    if (libraryId) {
      controlAttention.libs[libraryId] = true;
      delete controlAttention.pendingDevservers[devserverId];
    }
  }
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
  controlAttention.pendingDevservers = {};
}
