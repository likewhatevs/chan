// The launcher's HTTP client against the chan-library loopback server.
//
// The launcher is a pure HTTP client: it never opens native windows, never
// dials a devserver, never parses an opaque window id. Every type here
// mirrors a struct the library serializes, so the field names ARE the wire
// (the server pins them with byte tests).

// ---- The authoritative window record ------------------------------------
// What the library serves at GET /api/library/windows. The launcher renders
// these rows; `cs window list` and the desktop Window menu render the same
// rows off the same feed. Clients never parse `window_id` or `title`;
// remote-row decoration is recomposed from `kind`, `ordinal`, and
// `workspace_path`.

export type WindowKind = "terminal" | "workspace";

export interface WindowRecord {
  /** Library-minted, persisted, opaque, stable. The reconciliation key. Never parsed. */
  window_id: string;
  /** "local" (the baked-in local-disk library) or "lib-<16hex>" (a devserver). */
  library_id: string;
  kind: WindowKind;
  /** Library-composed, persisted, auto-derived (local perspective). Never parsed. */
  title: string;
  /** Per-(kind, workspace/library) "Window N"; library-owned, persisted. */
  ordinal: number;
  /** kind=workspace: full root path. null for kind=terminal. */
  workspace_path: string | null;
  /** Route prefix of the tenant serving this window's content. */
  prefix: string;
  /** Per-tenant bearer for `prefix`; empty when the owning tenant is off. */
  token: string;
  /** A durable library record exists (survives disconnect AND library restart). */
  persisted: boolean;
  /** A /ws socket tagged with window_id is live right now. */
  connected: boolean;
}

/** Full snapshot pushed on connect and on every change over the watch socket. */
export interface WindowSet {
  windows: WindowRecord[];
}

// ---- The workspace registry ---------------------------------------------

/** A registered workspace (a local folder) in the library registry. */
export interface WorkspaceEntry {
  /** Registry-assigned stable id (the server owns the scheme). */
  workspace_id: string;
  /** Canonical filesystem path of the workspace root. */
  path: string;
  /** Optional display label; empty falls back to the path basename. */
  label: string;
  /** Tenant is currently served (on) vs registered-but-off. */
  on: boolean;
}

// ---- The devserver registry ---------------------------------------------
// A devserver is a remote library the desktop dials out to. The launcher
// CRUDs the registry over HTTP (so the browser can manage it); the dial-out
// and native-window reconciliation are desktop-internal. The token is
// write-only over the wire: POST/PUT accept it, GET never echoes it back;
// `has_token` reports whether one is stored.

export interface DevserverEntry {
  /** Stable registry id used for row actions and the connection-state map. */
  id: string;
  /**
   * The full devserver URL the desktop dials, scheme included
   * (`https://box.example.com:8787`). The scheme is load-bearing: the dial path
   * branches on it, and the port defaults from it (https→443, http→80) when the
   * URL omits one.
   */
  url: string;
  /** Optional user label for the launcher section header. */
  label: string;
  /** Optional connect command the control terminal runs (e.g. an ssh forward). */
  script: string;
  /** Whether a bearer token is stored (the value is never returned). */
  has_token: boolean;
  /**
   * The library id this devserver is assigned once known, joining its window
   * rows in the feed to the user's name for it (the "↗ + name" remote label).
   * null before the devserver's first connect, when no library id exists yet.
   */
  library_id: string | null;
}

/** Write payload for add/edit devserver. `token` absent on edit leaves it unchanged. */
export interface DevserverInput {
  /** The full devserver URL (scheme included); validated `scheme://host[:port]`. */
  url: string;
  label?: string;
  script?: string;
  /** Bearer for a proxied/gateway devserver, so the user connects without scraping. */
  token?: string;
}

// ---- The client surface -------------------------------------------------
// One interface, implemented twice: the live HTTP client below, and the
// in-memory mock (`./mock`) the launcher runs against until the handlers are
// deployed. `watchWindows` returns an unsubscribe handle.

export interface LibraryApi {
  listWorkspaces(): Promise<WorkspaceEntry[]>;
  addLocalWorkspace(path: string): Promise<WorkspaceEntry>;
  setWorkspaceOn(id: string, on: boolean): Promise<void>;
  removeWorkspace(id: string): Promise<void>;
  listDevservers(): Promise<DevserverEntry[]>;
  addDevserver(input: DevserverInput): Promise<DevserverEntry>;
  updateDevserver(id: string, input: DevserverInput): Promise<DevserverEntry>;
  removeDevserver(id: string): Promise<void>;
  listWindows(): Promise<WindowRecord[]>;
  watchWindows(onSet: (set: WindowSet) => void): () => void;
  /** Mint a window of the local library (client supplies the kind; the library
   * supplies the id + persists). A terminal window has no workspace_path. */
  createWindow(kind: WindowKind, workspacePath?: string): Promise<WindowRecord>;
}

/** A non-2xx response, carrying the status and the server's text body. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    body: string,
  ) {
    super(body || `HTTP ${status}`);
    this.name = "ApiError";
  }
}

// The bearer the library hands the SPA. On loopback the launcher is served
// same-origin with a `?t=<token>` query (mirrors web/'s transport); a plain
// browser test passes it the same way. Empty means same-origin with no
// bearer (the local loopback default).
function authToken(): string {
  return new URLSearchParams(location.search).get("t") ?? "";
}

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
  const headers: Record<string, string> = {};
  const token = authToken();
  if (token) headers.authorization = `Bearer ${token}`;
  if (body !== undefined) headers["content-type"] = "application/json";
  const res = await fetch(path, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new ApiError(res.status, text);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

/** The live HTTP client. Ships once the /api/library/* handlers are deployed. */
export const liveApi: LibraryApi = {
  listWorkspaces: () => req("GET", "/api/library/workspaces"),
  addLocalWorkspace: (path) => req("POST", "/api/library/workspaces", { path }),
  setWorkspaceOn: (id, on) =>
    req("POST", `/api/library/workspaces/${encodeURIComponent(id)}/${on ? "on" : "off"}`),
  removeWorkspace: (id) => req("DELETE", `/api/library/workspaces/${encodeURIComponent(id)}`),
  listDevservers: () => req("GET", "/api/library/devservers"),
  addDevserver: (input) => req("POST", "/api/library/devservers", input),
  updateDevserver: (id, input) =>
    req("PUT", `/api/library/devservers/${encodeURIComponent(id)}`, input),
  removeDevserver: (id) => req("DELETE", `/api/library/devservers/${encodeURIComponent(id)}`),
  listWindows: () => req("GET", "/api/library/windows"),
  watchWindows: (onSet) => {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const token = authToken();
    const q = token ? `?t=${encodeURIComponent(token)}` : "";
    const ws = new WebSocket(`${proto}//${location.host}/api/library/windows/watch${q}`);
    ws.onmessage = (ev) => {
      try {
        onSet(JSON.parse(ev.data) as WindowSet);
      } catch {
        // A malformed frame is dropped; the next full snapshot self-heals.
      }
    };
    return () => ws.close();
  },
  createWindow: (kind, workspacePath) =>
    req("POST", "/api/library/windows", {
      kind,
      ...(workspacePath ? { workspace_path: workspacePath } : {}),
    }),
};
