// The devserver control-terminal-closed survey.
//
// A connected devserver is reachable only while its CONTROL terminal — the
// terminal running its connect command (e.g. an ssh -L forward) — stays alive.
// When that command exits while connected, the desktop emits
// `devserver-control-closed` carrying the devserver id; the connection is now
// dead. This singleton modal surveys the user — Re-run the command (reconnect),
// Edit it first, or Abandon the devserver — restoring the native launcher's old
// `runControlTerminalClosedSurvey` (dropped in the SPA migration, commit
// 151f1a7b). Desktop-only: the driving event never fires in a plain browser.
//
// Disconnect division (ratified — dev/v0.47.0/team/tasks/contract-Lead-bugB-disconnect.md):
// the SPA DRIVES reconnect/disconnect through the existing HTTP devserver
// actions. The desktop only emits the raw event (it can't know re-run vs
// abandon), so the user's choice is communicated by the call the SPA makes —
// Re-run = disconnect+connect, Edit = disconnect+edit form, Abandon = disconnect.
// Two pieces the SPA does NOT own:
//   - The stuck Control window record is reaped server-side on
//     PTY exit, uniformly for every outcome incl. Dismiss — neither this SPA nor
//     the desktop reaps it.
//   - The desktop flips the devserver `connected:false` UNCONDITIONALLY on the
//     event, so a dead devserver never shows connected even on Dismiss/no-response
//     (Abandon's explicit disconnect is then idempotent with that flip).

import { library, connectDevserver, disconnectDevserver, reportError } from "./library.svelte";
import { openEditDevserver } from "./dialog.svelte";

interface ControlClosedState {
  open: boolean;
  id: string;
  name: string;
  busy: boolean;
}

export const controlClosed = $state<ControlClosedState>({
  open: false,
  id: "",
  name: "",
  busy: false,
});

/** The user's name for a devserver id, for the survey title. Falls back through
 * label → host:port → a generic noun when the registry row is gone (a race). */
function devserverName(id: string): string {
  const ds = library.devservers.find((d) => d.id === id);
  if (!ds) return "The devserver";
  return ds.label.trim() || `${ds.host}:${ds.port}`;
}

/** Pull the devserver id out of the `devserver-control-closed` payload. The
 * desktop emits the bare String id (Tauri serializes it as a JSON string); the
 * old native handler also tolerated an `{ id }` object, so accept both shapes —
 * the listener works whichever the contract settles on. Returns null for an
 * unrecognized payload. */
export function controlClosedId(payload: unknown): string | null {
  if (typeof payload === "string") return payload || null;
  if (payload && typeof payload === "object") {
    const id = (payload as { id?: unknown }).id;
    if (typeof id === "string") return id || null;
  }
  return null;
}

/** Dispatch a raw `devserver-control-closed` payload to the survey. */
export function onControlClosedEvent(payload: unknown): void {
  const id = controlClosedId(payload);
  if (id) handleControlClosed(id);
}

/** Open the survey for a devserver whose control terminal just closed. One modal
 * at a time: the exit watcher and the empty-window close can both fire for a
 * single devserver, so a second event while a survey is open is ignored (the
 * native handler deduped per id; the single in-SPA modal subsumes that). */
export function handleControlClosed(id: string): void {
  if (controlClosed.open) return;
  controlClosed.id = id;
  controlClosed.name = devserverName(id);
  controlClosed.busy = false;
  controlClosed.open = true;
}

function close(): void {
  controlClosed.open = false;
  controlClosed.busy = false;
  controlClosed.id = "";
  controlClosed.name = "";
}

/** Re-run: clear the dead connection, then reconnect (the desktop re-runs the
 * connect command). The pre-disconnect is tolerant — the connection may already
 * be gone — so the reconnect still goes through; a reconnect failure surfaces in
 * the launcher banner. */
export async function rerunControlClosed(): Promise<void> {
  if (controlClosed.busy || !controlClosed.id) return;
  const id = controlClosed.id;
  controlClosed.busy = true;
  try {
    await disconnectDevserver(id).catch(() => {});
    await connectDevserver(id);
  } catch (e) {
    reportError(e);
  } finally {
    close();
  }
}

/** Edit: clear the dead connection (so the edit form opens editable — it is
 * read-only while connected), close the survey, and open the devserver edit
 * form prefilled with the current connect command. The user changes it and
 * reconnects from the row (the "edit before re-run" affordance). */
export async function editControlClosed(): Promise<void> {
  if (controlClosed.busy || !controlClosed.id) return;
  const id = controlClosed.id;
  controlClosed.busy = true;
  await disconnectDevserver(id).catch(() => {});
  close();
  // Re-look-up AFTER the disconnect refresh so the entry carries connected:false
  // and the edit form opens editable.
  const ds = library.devservers.find((d) => d.id === id);
  if (ds) openEditDevserver(ds);
}

/** Abandon: drop the dead connection (idempotent with the desktop's
 * unconditional `connected:false` flip on the event). The stale Control window
 * record itself is reaped server-side on PTY exit, not by
 * this disconnect, so it leaves the feed regardless. A genuine disconnect
 * failure surfaces in the banner (not swallowed — the disconnect IS the action
 * here). */
export async function abandonControlClosed(): Promise<void> {
  if (controlClosed.busy || !controlClosed.id) return;
  const id = controlClosed.id;
  controlClosed.busy = true;
  try {
    await disconnectDevserver(id);
  } catch (e) {
    reportError(e);
  } finally {
    close();
  }
}

/** Dismiss without acting (backdrop / Escape / ×). No SPA action — yet the dead
 * state still becomes correct without us: the desktop flips `connected:false`
 * unconditionally on the event, and the server reaps the
 * Control window on PTY exit. */
export function dismissControlClosed(): void {
  if (controlClosed.busy) return;
  close();
}
