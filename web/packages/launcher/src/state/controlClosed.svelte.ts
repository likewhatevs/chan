// Dispatch for the desktop's `devserver-control-closed` event.
//
// A connected devserver is reachable only while its CONTROL terminal -- the
// terminal running its connect command (e.g. an ssh -L forward) -- stays alive.
// When that command exits while connected, the desktop emits
// `devserver-control-closed` carrying the devserver id. The launcher flashes the
// control row's eye yellow (the amber "reconnecting" cue) for attention; the
// flash clears when the user acts on the window or the devserver reconnects.
// Desktop-only: the driving event never fires in a plain browser.

import { markControlAttention } from "./controlAttention.svelte";

/** Pull the devserver id out of a `devserver-control-closed` / `devserver-abandon`
 * payload. The desktop emits the bare String id (Tauri serializes it as a JSON
 * string); an `{ id }` object is also tolerated. Returns null for an
 * unrecognized payload. */
export function controlClosedId(payload: unknown): string | null {
  if (typeof payload === "string") return payload || null;
  if (payload && typeof payload === "object") {
    const id = (payload as { id?: unknown }).id;
    if (typeof id === "string") return id || null;
  }
  return null;
}

/** Dispatch a raw `devserver-control-closed` payload: flash the control row's eye
 * for attention. The flash lingers until the user acts on the window or the
 * devserver reconnects. */
export function onControlClosedEvent(payload: unknown): void {
  const id = controlClosedId(payload);
  if (id) markControlAttention(id);
}
