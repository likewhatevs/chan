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

/**
 * A workspace row in the launcher's workspace feed. Local rows are folders in
 * the local registry (`devserver_id` null); remote rows are workspaces a
 * connected devserver serves (`devserver_id` set), merged into the same feed so
 * the SPA groups them under their devserver. The field names ARE the wire (the
 * server's `LauncherWorkspace`).
 */
export interface WorkspaceEntry {
  /** Registry-assigned stable id (the server owns the scheme). */
  workspace_id: string;
  /** Canonical filesystem path of the workspace root. */
  path: string;
  /** Optional display label; empty falls back to the path basename. */
  label: string;
  /** Tenant is currently served (on) vs registered-but-off. */
  on: boolean;
  /** The library serving this row: null/"local" for local rows; the remote
   * library id for a devserver-served row. */
  library_id: string | null;
  /** The devserver serving this row: null for a local row; the devserver
   * registry id for a remote row (the row's group key + route target). */
  devserver_id: string | null;
  /** The mounted route prefix. For a local row it equals the workspace_id (the
   * slash-free local prefix; local on/off/rm route by workspace_id). For a
   * devserver row it is the remote mount prefix the devserver workspace
   * on/off/forget routes target. */
  prefix: string;
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
   * Optional per-library pane-highlight colour as a hex string (`#rrggbb`): the
   * desktop tints a window's active-pane highlight with its library's colour.
   * Absent/null falls back to the default accent. The add/edit dialog sets it;
   * the row shows it as a swatch.
   */
  color?: string | null;
  /**
   * The library id this devserver is assigned once known, joining its window
   * rows in the feed to the user's name for it (the "↗ + name" remote label).
   * null before the devserver's first connect, when no library id exists yet.
   */
  library_id: string | null;
  /**
   * Whether the desktop currently holds a live connection to this devserver.
   * Drives Connect vs Disconnect and gates Edit read-only (the backend rejects
   * edits while connected). Always false on a surface with no desktop bridge.
   */
  connected: boolean;
}

/** Write payload for add/edit devserver. `token` absent on edit leaves it unchanged. */
export interface DevserverInput {
  /** The full devserver URL (scheme included); validated `scheme://host[:port]`. */
  url: string;
  label?: string;
  script?: string;
  /** Bearer for a proxied/gateway devserver, so the user connects without scraping. */
  token?: string;
  /** Pane-highlight colour (hex `#rrggbb`); absent/null = the default accent. */
  color?: string | null;
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
  /** The local library's pane-highlight colour (hex `#rrggbb`), or null for the
   * default accent. Served on every surface (no store → null). */
  getLocalColor(): Promise<string | null>;
  /** Set the local library's pane-highlight colour; null clears it to the
   * default accent. Loopback-only (403 read-only on a read-only surface). */
  setLocalColor(color: string | null): Promise<void>;
  /** Tell the desktop to connect a devserver (run its connect command + dial
   * the URL). A pure desktop action: a surface with no desktop bridge answers
   * 409, so the launcher only offers it where window-ops are available. */
  connectDevserver(id: string): Promise<void>;
  /** Tear down the desktop's live connection to a devserver (its windows leave
   * the feed). A desktop action; a surface with no desktop bridge answers 409. */
  disconnectDevserver(id: string): Promise<void>;
  /** Open a terminal window on a connected devserver (desktop action, 409 with
   * no bridge). */
  openDevserverTerminal(id: string): Promise<void>;
  /** Open a window onto one of a connected devserver's served workspaces by its
   * remote path (desktop action, 409 with no bridge). */
  openDevserverWorkspace(id: string, path: string): Promise<void>;
  /** Turn a connected devserver's served workspace on/off by its mounted prefix
   * (desktop action, 409 with no bridge). An unforced off of a workspace that
   * still has live terminal sessions answers 409 `{error:"live_terminals",
   * active_terminals:N}`; pass `force` to tear them down and turn off anyway. */
  setDevserverWorkspaceOn(id: string, prefix: string, on: boolean, force?: boolean): Promise<void>;
  /** Forget (unmount + drop) a connected devserver's served workspace by its
   * mounted prefix (desktop action, 409 with no bridge). */
  forgetDevserverWorkspace(id: string, prefix: string): Promise<void>;
  /** Open the desktop's native folder picker; resolves to the chosen absolute
   * path, or null if the user cancelled. A pure desktop action (no desktop
   * bridge → 409), offered only where window-ops are available. */
  pickFolder(): Promise<string | null>;
  listWindows(): Promise<WindowRecord[]>;
  watchWindows(onSet: (set: WindowSet) => void): () => void;
  /** Mint a window of the local library (client supplies the kind; the library
   * supplies the id + persists). A terminal window has no workspace_path. */
  createWindow(kind: WindowKind, workspacePath?: string): Promise<WindowRecord>;
  /** Open (focus a live window / un-hide a buried one) via the desktop window
   * bridge. Rejects on a surface with no desktop attached. */
  openWindow(id: string): Promise<void>;
  /** Hide (bury) a window via the desktop window bridge — notification-free,
   * unlike the OS close button. Rejects with no desktop attached. */
  hideWindow(id: string): Promise<void>;
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

/**
 * The live-terminal count carried by an unforced devserver-workspace off that
 * was refused because the workspace still has live terminal sessions. Returns
 * `active_terminals` when `e` is an `ApiError` whose 409 body parses to
 * `{error:"live_terminals", active_terminals:N}`, else null — so the launcher
 * can confirm-and-retry only that case and let a plain `NO_DESKTOP` 409 (whose
 * body is not that JSON) fall through to the generic error banner.
 */
export function liveTerminalsCount(e: unknown): number | null {
  if (!(e instanceof ApiError) || e.status !== 409) return null;
  try {
    const body = JSON.parse(e.message) as { error?: unknown; active_terminals?: unknown };
    if (body.error === "live_terminals" && typeof body.active_terminals === "number") {
      return body.active_terminals;
    }
  } catch {
    // Not JSON (e.g. the plain `NO_DESKTOP` string body) → not the live case.
  }
  return null;
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
  getLocalColor: () =>
    req<{ color: string | null }>("GET", "/api/library/local-color").then((r) => r.color),
  setLocalColor: (color) => req("PUT", "/api/library/local-color", { color }),
  connectDevserver: (id) =>
    req("POST", `/api/library/devservers/${encodeURIComponent(id)}/connect`),
  disconnectDevserver: (id) =>
    req("POST", `/api/library/devservers/${encodeURIComponent(id)}/disconnect`),
  openDevserverTerminal: (id) =>
    req("POST", `/api/library/devservers/${encodeURIComponent(id)}/terminal`),
  // The devserver-workspace ops carry the remote mount `prefix` / `path` in the
  // JSON body, not a path segment: a mount prefix can hold characters axum's
  // Path extractor + intervening (gateway) proxies mangle. on/off/forget are
  // distinct POST routes; forget is POST (a DELETE body is poorly supported).
  openDevserverWorkspace: (id, path) =>
    req("POST", `/api/library/devservers/${encodeURIComponent(id)}/workspaces/open`, { path }),
  // `on` posts just the prefix; `off` carries `force` so an unforced off can
  // answer 409 live_terminals and the launcher retry the same route forced.
  setDevserverWorkspaceOn: (id, prefix, on, force) =>
    req(
      "POST",
      `/api/library/devservers/${encodeURIComponent(id)}/workspaces/${on ? "on" : "off"}`,
      on ? { prefix } : { prefix, force: force ?? false },
    ),
  forgetDevserverWorkspace: (id, prefix) =>
    req("POST", `/api/library/devservers/${encodeURIComponent(id)}/workspaces/forget`, { prefix }),
  pickFolder: () => req("POST", "/api/library/fs/pick-folder"),
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
  openWindow: (id) => req("POST", `/api/library/windows/${encodeURIComponent(id)}/open`),
  hideWindow: (id) => req("POST", `/api/library/windows/${encodeURIComponent(id)}/hide`),
};
