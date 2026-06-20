// An in-memory backend implementing the same wire as the live client. The
// launcher runs against this until the /api/library/* handlers are deployed,
// which keeps the whole SPA browser-testable with no backend. It seeds a
// local library plus one devserver so every surface (registry rows, the
// two-choice dialog, the window feed with both local and remote libraries)
// has something real to render. Mutations notify the watch subscribers, so
// the feed updates live when a workspace toggles or a devserver is added.

import type {
  DevserverEntry,
  LibraryApi,
  WindowRecord,
  WindowSet,
  WorkspaceEntry,
} from "./library";

// The mock stores the bearer alongside the public record; the public shape
// (returned by listDevservers) never carries it.
interface MockDevserver extends DevserverEntry {
  token: string;
}

const DS_LIBRARY_ID = "lib-7f3a9c21b40d8e65";

let nextWs = 3;
let nextDs = 3;

const workspaces: WorkspaceEntry[] = [
  { workspace_id: "ws-1", path: "/Users/fiorix/notes", label: "", on: true },
  { workspace_id: "ws-2", path: "/Users/fiorix/work/journal", label: "Journal", on: false },
];

const devservers: MockDevserver[] = [
  {
    id: "ds-1",
    host: "box.example.com",
    port: 8787,
    label: "prod",
    script: "ssh box.example.com -L 8787:localhost:8787 chan devserver",
    has_token: true,
    token: "tok_seeded_prod",
    library_id: DS_LIBRARY_ID,
  },
];

const windows: WindowRecord[] = [
  {
    window_id: "w-local-term-1",
    library_id: "local",
    kind: "terminal",
    title: "🏠 Terminal Window 1",
    ordinal: 1,
    workspace_path: null,
    prefix: "t/local-1",
    token: "tok_local_term",
    persisted: true,
    connected: true,
  },
  {
    window_id: "w-local-notes-1",
    library_id: "local",
    kind: "workspace",
    title: "🏠 notes Window 1",
    ordinal: 1,
    workspace_path: "/Users/fiorix/notes",
    prefix: "w/notes",
    token: "tok_local_notes",
    persisted: true,
    connected: true,
  },
  {
    window_id: "w-ds1-term-1",
    library_id: DS_LIBRARY_ID,
    kind: "terminal",
    title: "🏠 Terminal Window 1",
    ordinal: 1,
    workspace_path: null,
    prefix: "t/ds1-1",
    token: "",
    persisted: true,
    connected: false,
  },
  {
    window_id: "w-ds1-api-1",
    library_id: DS_LIBRARY_ID,
    kind: "workspace",
    title: "🏠 api Window 1",
    ordinal: 1,
    workspace_path: "/srv/api",
    prefix: "w/api",
    token: "tok_ds1_api",
    persisted: true,
    connected: true,
  },
];

const subscribers = new Set<(set: WindowSet) => void>();

function notify(): void {
  const set: WindowSet = { windows: windows.map((w) => ({ ...w })) };
  for (const fn of subscribers) fn(set);
}

function publicDevserver(ds: MockDevserver): DevserverEntry {
  // Never echo the token; report only whether one is stored (write-only wire).
  return {
    id: ds.id,
    host: ds.host,
    port: ds.port,
    label: ds.label,
    script: ds.script,
    has_token: ds.has_token,
    library_id: ds.library_id,
  };
}

// A small async hop so the SPA exercises its loading paths the way it will
// against the network, and so optimistic UI has a frame to settle.
function tick<T>(value: T): Promise<T> {
  return Promise.resolve(value);
}

export const mockApi: LibraryApi = {
  listWorkspaces: () => tick(workspaces.map((w) => ({ ...w }))),

  addLocalWorkspace: (path) => {
    const entry: WorkspaceEntry = { workspace_id: `ws-${nextWs++}`, path, label: "", on: true };
    workspaces.push(entry);
    return tick({ ...entry });
  },

  setWorkspaceOn: (id, on) => {
    const ws = workspaces.find((w) => w.workspace_id === id);
    if (ws) ws.on = on;
    // An off workspace's windows lose their tenant token; on restores it.
    for (const w of windows) {
      if (w.kind === "workspace" && w.library_id === "local" && w.workspace_path === ws?.path) {
        w.token = on ? `tok_${id}` : "";
        w.connected = on && w.connected;
      }
    }
    notify();
    return tick(undefined);
  },

  removeWorkspace: (id) => {
    const i = workspaces.findIndex((w) => w.workspace_id === id);
    if (i >= 0) workspaces.splice(i, 1);
    return tick(undefined);
  },

  listDevservers: () => tick(devservers.map(publicDevserver)),

  addDevserver: (input) => {
    const ds: MockDevserver = {
      id: `ds-${nextDs++}`,
      host: input.host,
      port: input.port,
      label: input.label ?? "",
      script: input.script ?? "",
      has_token: !!input.token,
      token: input.token ?? "",
      // No library id until the desktop connects this devserver for the first time.
      library_id: null,
    };
    devservers.push(ds);
    return tick(publicDevserver(ds));
  },

  updateDevserver: (id, input) => {
    const ds = devservers.find((d) => d.id === id);
    if (!ds) throw new Error(`unknown devserver ${id}`);
    ds.host = input.host;
    ds.port = input.port;
    ds.label = input.label ?? "";
    ds.script = input.script ?? "";
    // A token omitted on edit leaves the stored one unchanged (the write-only
    // contract): only a non-empty token replaces it.
    if (input.token) {
      ds.token = input.token;
      ds.has_token = true;
    }
    return tick(publicDevserver(ds));
  },

  removeDevserver: (id) => {
    const i = devservers.findIndex((d) => d.id === id);
    if (i >= 0) devservers.splice(i, 1);
    return tick(undefined);
  },

  listWindows: () => tick(windows.map((w) => ({ ...w }))),

  createWindow: (kind, workspacePath) => {
    const localTerminals = windows.filter(
      (w) => w.library_id === "local" && w.kind === "terminal",
    ).length;
    const ordinal = kind === "terminal" ? localTerminals + 1 : 1;
    const base = workspacePath ? workspacePath.split("/").filter(Boolean).pop() : null;
    const rec: WindowRecord = {
      window_id: `w-local-${kind}-${windows.length + 1}`,
      library_id: "local",
      kind,
      title: kind === "terminal" ? `🏠 Terminal Window ${ordinal}` : `🏠 ${base} Window ${ordinal}`,
      ordinal,
      workspace_path: kind === "workspace" ? (workspacePath ?? null) : null,
      prefix: `t/local-${windows.length + 1}`,
      token: "tok_local",
      persisted: true,
      connected: true,
    };
    windows.push(rec);
    notify();
    return tick({ ...rec });
  },

  watchWindows: (onSet) => {
    subscribers.add(onSet);
    // Full snapshot on subscribe, mirroring the live watch socket.
    onSet({ windows: windows.map((w) => ({ ...w })) });
    return () => {
      subscribers.delete(onSet);
    };
  },
};
