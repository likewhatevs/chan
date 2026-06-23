// The launcher's reactive view of the library: the workspace registry, the
// devserver registry, and the live window feed. Mutations go through the
// backend and re-list the affected registry so the UI matches the server of
// record; the window feed updates from the watch subscription.

import { backend } from "../api/backend";
import type { DevserverEntry, DevserverInput, WindowRecord, WorkspaceEntry } from "../api/library";
import {
  beginPending,
  clearPending,
  dsKey,
  reconcile,
  servedKey,
  wsKey,
  type PendingTarget,
} from "./pending.svelte";

interface LibraryState {
  workspaces: WorkspaceEntry[];
  devservers: DevserverEntry[];
  windows: WindowRecord[];
  loading: boolean;
  error: string | null;
}

export const library = $state<LibraryState>({
  workspaces: [],
  devservers: [],
  windows: [],
  loading: false,
  error: null,
});

let unwatch: (() => void) | null = null;

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** Surface a failed action in the launcher's error banner. Components catch
 * their action rejections and route them here; the throwing actions stay
 * uniform (so bulk loops can count per-item failures). */
export function reportError(e: unknown): void {
  library.error = errorText(e);
}

export function clearError(): void {
  library.error = null;
}

// Build the per-row state map the pending store reconciles against (the same
// keys the action handlers + the spinner UI use), then clear any in-flight
// marker whose row has reached its target / is gone / has timed out. Called
// after every registry refresh + on loadLibrary, so a finished or abandoned
// on/off / connect stops spinning and the row shows its real state.
function reconcilePending(): void {
  const current: Record<string, PendingTarget> = {};
  for (const w of library.workspaces) {
    if (w.devserver_id === null) current[wsKey(w.workspace_id)] = w.on ? "on" : "off";
    else current[servedKey(w.devserver_id, w.prefix)] = w.on ? "on" : "off";
  }
  for (const d of library.devservers) {
    current[dsKey(d.id)] = d.connected ? "connected" : "disconnected";
  }
  reconcile(current);
}

/** Load both registries and subscribe to the window feed (idempotent watch). */
export async function loadLibrary(): Promise<void> {
  library.loading = true;
  library.error = null;
  try {
    const [workspaces, devservers] = await Promise.all([
      backend.listWorkspaces(),
      backend.listDevservers(),
    ]);
    library.workspaces = workspaces;
    library.devservers = devservers;
    // On mount/reload: clear any persisted marker the real state already
    // satisfies (the op finished while we were away), so the spinner picks up
    // the latest state and only survives for rows still genuinely in-flight.
    reconcilePending();
  } catch (e) {
    library.error = errorText(e);
  } finally {
    library.loading = false;
  }
  if (!unwatch) {
    try {
      unwatch = backend.watchWindows((set) => {
        library.windows = set.windows;
        // The feed also fires on workspace mount/unmount (chan open / on / off)
        // and on a devserver connect/disconnect (its windows enter/leave + its
        // served-workspace rows merge in/out, and its `connected` flag flips),
        // so re-fetch both registries to reflect the new state live — no manual
        // reload, even when the change is out-of-band (desktop menu / CLI /
        // another launcher). Each is coalesced so a burst of window pushes
        // collapses to at most one extra GET apiece.
        void refreshWorkspacesLive();
        void refreshDevserversLive();
      });
    } catch {
      // The window feed is best-effort: a host without WebSocket or a failed
      // connection must not break loading the registries.
    }
  }
}

export function stopWatching(): void {
  unwatch?.();
  unwatch = null;
}

async function refreshWorkspaces(): Promise<void> {
  library.workspaces = await backend.listWorkspaces();
  reconcilePending();
}

// The live re-fetch the window-watch feed drives. The feed pushes a full
// snapshot on every window change, so bursts are coalesced: while a re-fetch
// is in flight, a later push just flags one more run, and the in-flight call
// re-runs once when it lands. No timer, so nothing leaks between tests, and a
// transient list error is swallowed — the next push (or a manual reload) heals.
let liveRefreshing = false;
let liveRefreshPending = false;

async function refreshWorkspacesLive(): Promise<void> {
  if (liveRefreshing) {
    liveRefreshPending = true;
    return;
  }
  liveRefreshing = true;
  try {
    do {
      liveRefreshPending = false;
      library.workspaces = await backend.listWorkspaces();
    } while (liveRefreshPending);
    reconcilePending();
  } catch {
    // Best-effort: a failed live re-fetch must not tear down the feed.
  } finally {
    liveRefreshing = false;
  }
}

async function refreshDevservers(): Promise<void> {
  library.devservers = await backend.listDevservers();
  reconcilePending();
}

// The live devserver re-fetch the window-watch feed drives, mirroring
// refreshWorkspacesLive: a connect/disconnect flips `connected` (Connect vs
// Disconnect) and changes which devservers' workspaces merge into the feed.
// Coalesced + best-effort for the same reasons (no leaked timer; a transient
// list error heals on the next push).
let liveDevserversRefreshing = false;
let liveDevserversRefreshPending = false;

async function refreshDevserversLive(): Promise<void> {
  if (liveDevserversRefreshing) {
    liveDevserversRefreshPending = true;
    return;
  }
  liveDevserversRefreshing = true;
  try {
    do {
      liveDevserversRefreshPending = false;
      library.devservers = await backend.listDevservers();
    } while (liveDevserversRefreshPending);
    reconcilePending();
  } catch {
    // Best-effort: a failed live re-fetch must not tear down the feed.
  } finally {
    liveDevserversRefreshing = false;
  }
}

export async function addLocalWorkspace(path: string): Promise<void> {
  await backend.addLocalWorkspace(path);
  await refreshWorkspaces();
}

export async function toggleWorkspace(id: string, on: boolean, force?: boolean): Promise<void> {
  const key = wsKey(id);
  beginPending(key, on ? "on" : "off");
  try {
    await backend.setWorkspaceOn(id, on, force);
  } catch (e) {
    clearPending(key); // stop the spinner; the error surfaces / the confirm opens
    throw e;
  }
  await refreshWorkspaces(); // reconcile clears the marker once on/off has landed
}

export async function removeWorkspace(id: string): Promise<void> {
  await backend.removeWorkspace(id);
  await refreshWorkspaces();
}

/** Open the desktop's native folder picker for the New-Workspace Folder field;
 * returns the chosen absolute path, or null on cancel / a non-desktop surface.
 * Throws on a real error so the dialog can surface it. */
export async function pickFolder(): Promise<string | null> {
  return (await backend.pickFolder()) ?? null;
}

/** Add (no id) or edit (id) a devserver; an empty `token` on edit is unchanged. */
export async function saveDevserver(input: DevserverInput, id?: string): Promise<void> {
  if (id) await backend.updateDevserver(id, input);
  else await backend.addDevserver(input);
  await refreshDevservers();
}

export async function removeDevserver(id: string): Promise<void> {
  await backend.removeDevserver(id);
  await refreshDevservers();
}

// The devserver bridge actions are desktop actions: a surface with no desktop
// bridge answers 409. They throw on failure (uniform with the workspace actions,
// so the bulk loop can count per-item failures); the per-row callers catch and
// route the error to the banner via reportError. Connect/disconnect re-list the
// devserver registry so the acting client's Connect/Disconnect flips at once
// (the watch push keeps it live for out-of-band changes); the window-minting
// actions (terminal / open) rely on the watch push alone.

/** Connect a devserver: the desktop runs its connect command and dials the URL.
 * Its windows + served workspaces then appear in the feed via the watch push. */
export async function connectDevserver(id: string): Promise<void> {
  const key = dsKey(id);
  beginPending(key, "connected");
  try {
    await backend.connectDevserver(id);
  } catch (e) {
    clearPending(key);
    throw e;
  }
  await refreshDevservers(); // reconcile clears the marker once connected
}

/** Disconnect a devserver: its windows + served-workspace rows leave the feed;
 * the registry entry stays so Connect can redial. */
export async function disconnectDevserver(id: string): Promise<void> {
  const key = dsKey(id);
  beginPending(key, "disconnected");
  try {
    await backend.disconnectDevserver(id);
  } catch (e) {
    clearPending(key);
    throw e;
  }
  await refreshDevservers(); // reconcile clears the marker once disconnected
}

/** Open a terminal window on a connected devserver. The window feed updates
 * through the watch subscription, so nothing to refresh here. */
export async function openDevserverTerminal(id: string): Promise<void> {
  await backend.openDevserverTerminal(id);
}

/** Open a window onto a connected devserver's served workspace by its remote
 * path. The window feed updates through the watch subscription. */
export async function openDevserverWorkspace(id: string, path: string): Promise<void> {
  await backend.openDevserverWorkspace(id, path);
}

/** Turn a connected devserver's served workspace on/off by its mounted prefix.
 * The merged workspace rows refresh through the watch push (the desktop bridges
 * its workspace-cache change into the library change-signal). An unforced off of
 * a workspace with live terminals throws an `ApiError` the caller maps to a
 * confirm dialog (see `liveTerminalsCount`); `force` retries past it. */
export async function setDevserverWorkspaceOn(
  id: string,
  prefix: string,
  on: boolean,
  force?: boolean,
): Promise<void> {
  const key = servedKey(id, prefix);
  beginPending(key, on ? "on" : "off");
  try {
    await backend.setDevserverWorkspaceOn(id, prefix, on, force);
  } catch (e) {
    clearPending(key); // stop the spinner; a 409 live-terminal opens the confirm
    throw e;
  }
  // No direct refresh here — the watch push drives refreshWorkspacesLive, whose
  // reconcile clears the marker once the served row flips.
}

/** Forget (unmount + drop) a connected devserver's served workspace by prefix. */
export async function forgetDevserverWorkspace(id: string, prefix: string): Promise<void> {
  await backend.forgetDevserverWorkspace(id, prefix);
}

/** Mint a new terminal window of the local library. The window feed updates
 * itself through the watch subscription, so there is nothing to refresh here. */
export async function openTerminal(): Promise<void> {
  await backend.createWindow("terminal");
}

/** Open a window onto an on workspace: mint a workspace window of the local
 * library (the desktop embed focuses an existing one for the same path). The
 * window feed updates through the watch subscription, so nothing to refresh. */
export async function openWorkspaceWindow(path: string): Promise<void> {
  await backend.createWindow("workspace", path);
}

/** Toggle a window's visibility (the feed's SHOW/HIDE Eye): hide it if it is
 * visible, otherwise open (un-hide/focus) it. Keyed on the server-persisted
 * `hidden` (Theme 5), not socket liveness — the toggle stays a bridge op
 * (`hideWindow`/`openWindow`); the desktop persists `hidden` at the bury/unbury
 * chokepoint, so the row moves between Open/Hidden on the feed round-trip. No
 * optimistic flip here — the feed reflects the live state after the watch push. */
export async function toggleWindow(w: WindowRecord): Promise<void> {
  if (w.hidden) await backend.openWindow(w.window_id);
  else await backend.hideWindow(w.window_id);
}

/** Focus a window (the feed's FOCUS action): openWindow focuses a live window
 * and un-hides + focuses a buried one (it is the only un-hide op), matching the
 * desired focus behavior either way. The feed updates through the watch push, so
 * there is nothing to refresh here. */
export async function focusWindow(w: WindowRecord): Promise<void> {
  await backend.openWindow(w.window_id);
}

/** The user's name for a remote library, joined by its library id. */
export function remoteLibraryName(libraryId: string): string | null {
  const ds = library.devservers.find((d) => d.library_id === libraryId);
  if (!ds) return null;
  return ds.label || `${ds.host}:${ds.port}`;
}
