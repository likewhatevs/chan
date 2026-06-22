// The launcher's reactive view of the library: the workspace registry, the
// devserver registry, and the live window feed. Mutations go through the
// backend and re-list the affected registry so the UI matches the server of
// record; the window feed updates from the watch subscription.

import { backend } from "../api/backend";
import type { DevserverEntry, DevserverInput, WindowRecord, WorkspaceEntry } from "../api/library";

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
  } catch (e) {
    library.error = errorText(e);
  } finally {
    library.loading = false;
  }
  if (!unwatch) {
    try {
      unwatch = backend.watchWindows((set) => {
        library.windows = set.windows;
        // The feed also fires on workspace mount/unmount (chan open / on / off),
        // so re-fetch the workspace list to reflect the new on-state live —
        // no manual reload. Coalesced so a burst of window pushes collapses to
        // at most one extra GET.
        void refreshWorkspacesLive();
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
  } catch {
    // Best-effort: a failed live re-fetch must not tear down the feed.
  } finally {
    liveRefreshing = false;
  }
}

async function refreshDevservers(): Promise<void> {
  library.devservers = await backend.listDevservers();
}

export async function addLocalWorkspace(path: string): Promise<void> {
  await backend.addLocalWorkspace(path);
  await refreshWorkspaces();
}

export async function toggleWorkspace(id: string, on: boolean): Promise<void> {
  await backend.setWorkspaceOn(id, on);
  await refreshWorkspaces();
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

/** Connect a devserver — a desktop action: the desktop runs its connect
 * command and dials the URL. Its windows then appear in the feed via the watch
 * push, so there is nothing to refresh here. A failure (a non-desktop surface
 * 409s, or the connect command/dial fails) surfaces in the error banner. */
export async function connectDevserver(id: string): Promise<void> {
  library.error = null;
  try {
    await backend.connectDevserver(id);
  } catch (e) {
    library.error = errorText(e);
  }
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

/** Toggle a window from the feed's status dot: hide it if it is connected,
 * otherwise open (focus/un-hide) it. The dot reflects the live feed after the
 * watch push, so there is no optimistic flip here. */
export async function toggleWindow(w: WindowRecord): Promise<void> {
  if (w.connected) await backend.hideWindow(w.window_id);
  else await backend.openWindow(w.window_id);
}

/** The user's name for a remote library, joined by its library id. */
export function remoteLibraryName(libraryId: string): string | null {
  const ds = library.devservers.find((d) => d.library_id === libraryId);
  if (!ds) return null;
  return ds.label || ds.url;
}
