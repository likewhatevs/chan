// API client. The typed surface lives here; wire mechanics
// (token plumbing, fetch shape, WebSocket reconnect) live in
// `./transport.ts`. Both `chan serve` and the Tauri desktop shell
// reach the same in-process server over loopback HTTP+WS, so there
// is one transport implementation, not a polymorphic seam.

import type {
  BuildInfo,
  ContentSearchResponse,
  FileResponse,
  FsGraphResponse,
  GlobalConfig,
  GraphEdge,
  GraphSnapshot,
  GraphView,
  HeadingRow,
  HealthResponse,
  InspectorPayload,
  IndexStatus,
  IndexingStateResponse,
  LanguageGraphResponse,
  MoveResponse,
  ReportFileStats,
  ReportPrefix,
  ResetMode,
  ResetResponse,
  SearchHit,
  TerminalRestartRequest,
  TerminalSpawnRequest,
  TerminalSpawnResponse,
  TreeEntry,
  DriveInfo,
  BubbleOverlayMode,
} from "./types";
import { ApiError } from "./errors";
import {
  apiPath,
  authToken as transportAuthToken,
  openWatch,
  request,
  withTokenQuery as transportWithTokenQuery,
} from "./transport";

export { ApiError } from "./errors";

/// Auth token for the current session (or null on a `--no-token`
/// server). Exported for the few call sites that build URLs outside
/// the api wrapper (e.g. `<img src>` in the editor's image extension,
/// the multipart upload helper).
export function authToken(): string | null {
  return transportAuthToken();
}

/// Append the auth token as a `?t=...` query. Use only for paths
/// that cannot carry an `Authorization` header (WebSocket upgrade,
/// `<img>` rendered by the browser).
export function withTokenQuery(path: string): string {
  return transportWithTokenQuery(path);
}

const BROWSER_SESSION_WINDOW_KEY = "chan.session.window";
let sessionStorageWarningShown = false;

function randomBrowserSessionId(): string {
  const bytes = new Uint8Array(4);
  if (typeof crypto !== "undefined" && crypto.getRandomValues) {
    crypto.getRandomValues(bytes);
  } else {
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = Math.floor(Math.random() * 256);
    }
  }
  return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
}

function browserSessionWindowId(): string | null {
  if (typeof window === "undefined") return null;
  try {
    const existing = window.sessionStorage.getItem(BROWSER_SESSION_WINDOW_KEY);
    if (existing?.trim()) return existing.trim();
    const generated = randomBrowserSessionId();
    window.sessionStorage.setItem(BROWSER_SESSION_WINDOW_KEY, generated);
    return generated;
  } catch {
    if (!sessionStorageWarningShown) {
      sessionStorageWarningShown = true;
      console.warn("[chan] sessionStorage unavailable; falling back to shared session key");
    }
    return null;
  }
}

/// Session blob key for this browser/webview window. chan-desktop
/// appends `?w=<window-label>` to each drive window URL; plain browser
/// tabs get a per-tab sessionStorage key so they do not overwrite
/// each other. If storage is unavailable we fall back to historical
/// shared `default` behavior.
export function sessionWindowId(): string {
  if (typeof window === "undefined") return "default";
  const raw = new URL(window.location.href).searchParams.get("w");
  const trimmed = raw?.trim();
  if (trimmed) return trimmed;
  return browserSessionWindowId() ?? "default";
}

export function sessionPath(): string {
  return `/api/session?w=${encodeURIComponent(sessionWindowId())}`;
}

function req<T>(
  method: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
  timeoutMs?: number,
): Promise<T> {
  return request<T>(method, path, body, signal, timeoutMs);
}

export const api = {
  drive: () => req<DriveInfo>("GET", "/api/drive"),
  /// Update the drive's display name in the global registry.
  /// Other preferences live in the global config and round-trip
  /// through /api/config.
  /// Empty / whitespace-only name clears the field.
  updatePreferences: (body: { name?: string | null }) =>
    req<DriveInfo>("PATCH", "/api/drive", body),
  /// Read the global per-user config (registry of known drives,
  /// default-drive path, preferences). Mounted by the Settings UI.
  config: () => req<GlobalConfig>("GET", "/api/config"),
  /// Replace the global config (whole-block PATCH).
  updateConfig: (body: GlobalConfig) =>
    req<GlobalConfig>("PATCH", "/api/config", body),
  /** Upload an image attachment. Multipart POST that the editor's `![`
   *  picker, drag-and-drop, and clipboard paste all funnel through.
   *  Returns the drive-relative path of the saved file.
   *
   *  `dir` is the drive-relative directory to save into. The editor
   *  passes the directory of the file being edited so uploads land
   *  next to it (markdown can then reference the file with `./name`).
   *  Null falls back to the server's configured `attachments_dir`. */
  uploadAttachment: async (
    file: File,
    dir: string | null = null,
  ): Promise<{ path: string }> => {
    // Multipart upload skips the JSON-shaped request() helper because
    // FormData cannot be JSON-encoded; we hit fetch directly and
    // reuse the same auth token.
    const form = new FormData();
    form.append("file", file);
    if (dir !== null) form.append("dir", dir);
    const headers: Record<string, string> = {};
    const tk = transportAuthToken();
    if (tk) headers.authorization = `Bearer ${tk}`;
    const res = await fetch(apiPath("/api/attachments"), { method: "POST", headers, body: form });
    if (!res.ok) {
      const text = await res.text().catch(() => res.statusText);
      throw new ApiError(res.status, text || res.statusText);
    }
    return (await res.json()) as { path: string };
  },
  /** Import contacts from a CSV. Multipart POST mirroring
   *  uploadAttachment: server runs the parser, drops one
   *  `chan.kind: contact` markdown note per contact under
   *  `destDir`, and returns a per-row outcome breakdown. The
   *  wizard surfaces these counts to the user. */
  importContacts: async (
    file: File,
    destDir: string,
    opts: { provider?: string; overwrite?: boolean } = {},
  ): Promise<{
    wrote: string[];
    overwrote: string[];
    skipped: Array<{ path: string; reason: string }>;
    failed: Array<{ name: string; reason: string }>;
    /** Non-fatal issues the server detected while parsing the
     *  request (e.g., unknown multipart parts that were ignored).
     *  Always present; empty when nothing unexpected showed up. */
    warnings: string[];
  }> => {
    const form = new FormData();
    form.append("file", file);
    form.append("dest_dir", destDir);
    form.append("provider", opts.provider ?? "google");
    form.append("overwrite", opts.overwrite ? "true" : "false");
    const headers: Record<string, string> = {};
    const tk = transportAuthToken();
    if (tk) headers.authorization = `Bearer ${tk}`;
    const res = await fetch(apiPath("/api/contacts/import"), {
      method: "POST",
      headers,
      body: form,
    });
    if (!res.ok) {
      const text = await res.text().catch(() => res.statusText);
      throw new ApiError(res.status, text || res.statusText);
    }
    const body = (await res.json()) as {
      wrote: string[];
      overwrote: string[];
      skipped: Array<{ path: string; reason: string }>;
      failed: Array<{ name: string; reason: string }>;
      warnings?: string[];
    };
    // `warnings` was added after the initial route shipped; tolerate
    // older servers that don't send it by defaulting to empty.
    return { ...body, warnings: body.warnings ?? [] };
  },
  /** List contact-kind notes for the editor `@` picker. Optional
   *  `q` is a case-insensitive substring filter against the
   *  contact's title, basename, AND any of its email addresses
   *  (so typing `alice` matches both "Alice Anderson" and a
   *  contact whose only `alice` is in `alice@example.com`); empty
   *  string returns the alphabetical head of the catalog. The
   *  returned `emails` is the contact's full address list, used
   *  by the picker to render a secondary line under the name. */
  contacts: (q = "", limit = 10) => {
    const qs = new URLSearchParams();
    if (q) qs.set("q", q);
    qs.set("limit", String(limit));
    return req<
      Array<{
        path: string;
        label: string;
        emails?: string[];
        /// `aliases:` array from the contact note's frontmatter
        /// (top-level, Obsidian convention). Empty when the contact
        /// has no alternate names. The mention `@@` trigger uses
        /// this to commit `@@<alias>` and the resolver maps any
        /// alias back to the contact file.
        aliases?: string[];
      }>
    >("GET", `/api/contacts?${qs.toString()}`);
  },
  list: (dir?: string | null) => {
    const qs = new URLSearchParams();
    if (dir !== undefined && dir !== null) qs.set("dir", dir);
    const suffix = qs.size > 0 ? `?${qs.toString()}` : "";
    return req<TreeEntry[]>("GET", `/api/files${suffix}`);
  },
  read: (path: string) => req<FileResponse>("GET", `/api/files/${encPath(path)}`),
  /// Persist `content` at `path`. When `expectedMtime` is provided,
  /// the server CAS-writes via Drive::write_text_if_unchanged and
  /// rejects with 409 + { current_mtime } if the on-disk mtime
  /// differs (an external edit landed since the client last read).
  /// Returns the new mtime so callers store it as the next CAS token.
  write: (path: string, content: string, expectedMtime?: number | null) =>
    req<{ mtime: number | null }>("PUT", `/api/files/${encPath(path)}`, {
      content,
      ...(expectedMtime !== undefined ? { expected_mtime: expectedMtime } : {}),
    }),
  create: (path: string, isDir: boolean, content?: string) =>
    req<void>("POST", "/api/files", { path, is_dir: isDir, content }),
  remove: (path: string) => req<void>("DELETE", `/api/files/${encPath(path)}`),
  move: (from: string, to: string) =>
    req<MoveResponse>("POST", "/api/move", { from, to }),
  /// Filename fuzzy search (the [[ autocomplete in the editor).
  /// Hits the renamed /api/search/files endpoint; the legacy
  /// /api/search alias still exists server-side for back-compat.
  /// `prefix` scopes the result set to files under that directory:
  /// the wiki-link picker passes the source file's git_repo root
  /// when applicable so suggestions stay project-bound.
  search: (q: string, limit = 10, prefix?: string | null) => {
    const params = new URLSearchParams({ q, limit: String(limit) });
    if (prefix) params.set("prefix", prefix);
    return req<SearchHit[]>("GET", `/api/search/files?${params}`);
  },
  /// Hybrid (BM25 + dense) content search. The backend silently
  /// picks hybrid (or BM25 when built without the `embeddings`
  /// feature); the previous user-facing mode picker was removed in
  /// favour of a single sensible default.
  searchContent: (q: string, opts: { limit?: number } = {}) => {
    const params = new URLSearchParams({ q });
    if (opts.limit !== undefined) params.set("limit", String(opts.limit));
    return req<ContentSearchResponse>(
      "GET",
      `/api/search/content?${params.toString()}`,
    );
  },
  /// Headings of a single file. The wiki-link bubble fetches this
  /// when the user types `#` after a resolved file target so
  /// suggestions filter against the file's outline.
  headings: (path: string) =>
    req<HeadingRow[]>("GET", `/api/headings/${encPath(path)}`),
  /// Incoming link edges (other files that link to `path`). Used by
  /// the editor's bottom-right status bar to show a backlink count;
  /// `.length` is enough for the count, but the full edge list is
  /// available for future "linked from" panels.
  backlinks: (path: string) =>
    req<GraphEdge[]>("GET", `/api/backlinks/${encPath(path)}`),
  /// chan-report per-file stats: language, SLOC, comments, blanks,
  /// complexity. 404 when the path isn't in the index (binary file,
  /// gitignored, or unknown language) — callers treat that as
  /// "no report for this file" rather than an error.
  reportFile: (path: string) =>
    req<ReportFileStats>(
      "GET",
      `/api/report/file?path=${encodeURIComponent(path)}`,
    ),
  /// chan-report directory roll-up: totals, by-language, and COCOMO.
  /// Empty `path` returns the whole-drive roll-up. The per-file
  /// array is dropped server-side; only the summary fields come
  /// back so big directories stay cheap to fetch.
  reportPrefix: (path: string) =>
    req<ReportPrefix>(
      "GET",
      `/api/report/prefix?path=${encodeURIComponent(path)}`,
    ),
  /// Resolve a wiki / markdown link target to the actual drive file
  /// + node kind. `target` is the path portion of the link (no
  /// `#anchor`); pass through path-encoded segments verbatim. The
  /// server returns 404 when no file matches any of the
  /// `path.md` / `path.txt` / `path` probes, so the client treats a
  /// missing resolve as "broken link" rather than an error.
  /// `kind` distinguishes contact-kind notes from generic docs so
  /// the editor can stamp `data-refkind` and render a kind-aware
  /// pill without re-parsing the target's frontmatter.
  resolveLink: (target: string) =>
    req<{ path: string; anchor?: string; kind: "file" | "contact" }>(
      "GET",
      `/api/resolve-link?target=${encodeURIComponent(target)}`,
    ),
  indexStatus: () => req<IndexStatus>("GET", "/api/index/status"),
  indexingState: () => req<IndexingStateResponse>("GET", "/api/indexing/state"),
  health: () => req<HealthResponse>("GET", "/api/health"),
  inspector: (path: string) =>
    req<InspectorPayload>(
      "GET",
      `/api/inspector?path=${encodeURIComponent(path)}`,
    ),
  /// Wipe and rebuild the search index from scratch. Returns when
  /// the rebuild has been kicked off; status moves through
  /// "building" via /api/index/status as files are reprocessed.
  indexRebuild: () => req<void>("POST", "/api/index/rebuild"),
  /// Compile-time identity (chan version + cargo features). Used by
  /// the Settings "About" footer.
  buildInfo: () => req<BuildInfo>("GET", "/api/build-info"),
  /// Reset the drive at one of three escalating levels. After a
  /// successful reset the caller should reload the window so cached
  /// drive info, file tree, and tabs resync; the server has done
  /// the work but in-app state still references the pre-reset world.
  storageReset: (mode: ResetMode) =>
    req<ResetResponse>("POST", "/api/storage/reset", { mode }),
  /// Read the persisted session payload. Server keys by `?w=<id>`;
  /// chan-desktop windows pass their unique window label in the page URL,
  /// while normal browser tabs use `default`. Returns `null` when none exists yet
  /// (server returns 204 → req() yields undefined → coerced to
  /// null for the caller's convenience).
  getSession: async (): Promise<unknown | null> => {
    const v = await req<unknown | undefined>("GET", sessionPath());
    return v ?? null;
  },
  /// Persist the session payload. Body shape is opaque to the
  /// server; the frontend sends `serializeLayout()` output.
  putSession: (body: unknown) =>
    req<void>("PUT", sessionPath(), body),
  links: () => req<GraphSnapshot>("GET", "/api/links"),
  /// Typed graph payload powering the graph view tab.
  graph: (opts: { scope?: "drive" | "directory" | "file"; path?: string; depth?: number } = {}) => {
    const params = new URLSearchParams();
    if (opts.scope) params.set("scope", opts.scope);
    if (opts.path) params.set("path", opts.path);
    if (opts.depth !== undefined) params.set("depth", String(opts.depth));
    const suffix = params.size > 0 ? `?${params.toString()}` : "";
    return req<GraphView>("GET", `/api/graph${suffix}`);
  },
  /// Language graph payload: language nodes connected to directory nodes.
  languageGraph: (opts: { depth?: number; language?: string } = {}) => {
    const params = new URLSearchParams();
    if (opts.depth !== undefined) params.set("depth", String(opts.depth));
    if (opts.language) params.set("language", opts.language);
    const suffix = params.size > 0 ? `?${params.toString()}` : "";
    return req<LanguageGraphResponse>("GET", `/api/graph/languages${suffix}`);
  },
  /// Filesystem graph payload: directories, files, symlinks, hardlinks,
  /// and ghost nodes. Distinct from the semantic markdown graph.
  fsGraph: (opts: { scope: "file" | "directory"; path: string; depth?: number }) =>
    req<FsGraphResponse>(
      "GET",
      `/api/fs-graph?scope=${encodeURIComponent(opts.scope)}&path=${encodeURIComponent(opts.path)}&depth=${encodeURIComponent(String(opts.depth ?? 1))}`,
    ),
  setTerminalWatcher: (sessionId: string, path: string) =>
    req<void>("POST", `/api/terminal/${encodeURIComponent(sessionId)}/watcher`, { path }),
  clearTerminalWatcher: (sessionId: string) =>
    req<void>("DELETE", `/api/terminal/${encodeURIComponent(sessionId)}/watcher`),
  writeTerminalEventReply: (
    sessionId: string,
    body: {
      id: string;
      type: "survey-reply";
      from: string;
      to: string;
      answers: Array<{ question_index: number; key: string }>;
      scope_grant: "one-shot" | "topic-session" | "topic-phase";
      follow_up?: boolean;
      note?: string;
    },
  ) =>
    req<void>(
      "POST",
      `/api/terminal/${encodeURIComponent(sessionId)}/event-reply`,
      body,
    ),
  spawnTerminal: (body: TerminalSpawnRequest) =>
    req<TerminalSpawnResponse>("POST", "/api/terminals", body),
  restartTerminal: (sessionId: string, body?: TerminalRestartRequest) =>
    req<void>("POST", `/api/terminals/${encodeURIComponent(sessionId)}/restart`, body),
  closeTerminal: (sessionId: string) =>
    req<void>("DELETE", `/api/terminals/${encodeURIComponent(sessionId)}`),
  setBubbleOverlayMode: async (mode: BubbleOverlayMode): Promise<void> => {
    const cfg = await req<GlobalConfig>("GET", "/api/config");
    if (cfg.preferences.bubble_overlay_mode === mode) return;
    await req<GlobalConfig>("PATCH", "/api/config", {
      ...cfg,
      preferences: { ...cfg.preferences, bubble_overlay_mode: mode },
    });
  },
};

/// Encode a path as a sequence of percent-encoded segments. We keep `/`
/// raw so axum's `*path` capture works.
function encPath(p: string): string {
  return p
    .split("/")
    .map((s) => encodeURIComponent(s))
    .join("/");
}

export type { WsStatus } from "./transport";

/// Open the watcher subscription. Auto-reconnects with capped
/// exponential backoff; the status callback drives the disconnect
/// overlay. Returns a disposer that closes the socket.
export function openWatchSocket(
  onEvent: (e: unknown) => void,
  onStatus?: (s: import("./transport").WsStatus) => void,
): () => void {
  return openWatch(onEvent, onStatus);
}
