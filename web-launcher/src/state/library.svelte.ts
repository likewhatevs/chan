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

/** The user's name for a remote library, joined by its library id. */
export function remoteLibraryName(libraryId: string): string | null {
  const ds = library.devservers.find((d) => d.library_id === libraryId);
  if (!ds) return null;
  return ds.label || ds.url;
}
