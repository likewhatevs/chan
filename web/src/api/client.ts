// API client. The typed surface lives here; wire mechanics
// (token plumbing, fetch shape, WebSocket reconnect) live in
// `./transport.ts`. Both `chan serve` and the Tauri desktop shell
// reach the same in-process server over loopback HTTP+WS, so there
// is one transport implementation, not a polymorphic seam.

import type {
  AnthropicModelsResponse,
  BuildInfo,
  ContentSearchResponse,
  FileResponse,
  GeminiModelsResponse,
  GlobalConfig,
  GraphSnapshot,
  GraphView,
  IndexStatus,
  LlmCompletionRequest,
  LlmCompletionResponse,
  LlmModelEntry,
  LlmStatus,
  LlmToolSpec,
  ResetMode,
  ResetResponse,
  SearchHit,
  TreeEntry,
  DriveInfo,
} from "./types";
import { ApiError } from "./errors";
import {
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
  /// Other preferences (fonts, assistant backend, attachments dir)
  /// live in the global config and round-trip through /api/config.
  /// Empty / whitespace-only name clears the field.
  updatePreferences: (body: { name?: string | null }) =>
    req<DriveInfo>("PATCH", "/api/drive", body),
  /// Read the global per-user config (registry of known drives,
  /// default-drive path, preferences). Mounted by the Settings UI.
  config: () => req<GlobalConfig>("GET", "/api/config"),
  /// Replace the global config (whole-block PATCH).
  updateConfig: (body: GlobalConfig) =>
    req<GlobalConfig>("PATCH", "/api/config", body),
  /// LLM backend status: which backend, model, and key state. Used
  /// by the Settings tab "Assistant" section to show ready / not
  /// ready and where the user should set their key.
  llmStatus: () => req<LlmStatus>("GET", "/api/llm/status"),
  /// One-shot assistant call. The whole conversation lives in
  /// `messages`; the server forwards to the configured backend.
  /// `signal` lets the caller abort a slow request (the Stop
  /// button in the inline-assist panel uses this); aborting drops
  /// the connection, axum sees the disconnect and the upstream
  /// (Anthropic / Ollama) request gets cancelled too.
  ///
  /// No client-side timeout: tool-use loops and slow models can
  /// legitimately run for minutes. The user-facing Stop button
  /// (via `signal`) is the cancellation path; the server enforces
  /// its own upstream deadlines.
  llmComplete: (body: LlmCompletionRequest, signal?: AbortSignal) =>
    req<LlmCompletionResponse>("POST", "/api/llm/complete", body, signal, 0),
  /** Tool catalog: the server's `default_tools()` list. */
  llmTools: () => req<LlmToolSpec[]>("GET", "/api/llm/tools"),
  /** Persist the Anthropic API key to the OS keychain. Surfaces
   *  through `llmStatus().key.source = "keychain"` once stored.
   *  Server returns 503 when the keychain backend isn't reachable
   *  (headless box, locked keychain); the Settings UI hides the
   *  call site in that case. */
  setAnthropicKey: (key: string) =>
    req<void>("PUT", "/api/llm/keys/anthropic", { key }),
  /** Drop the keychain entry. Idempotent. Leaves the env var and
   *  `~/.config/chan/api-keys.toml` untouched. */
  clearAnthropicKey: () => req<void>("DELETE", "/api/llm/keys/anthropic"),
  /** Same shape as setAnthropicKey, for the Google Gemini key. */
  setGeminiKey: (key: string) =>
    req<void>("PUT", "/api/llm/keys/gemini", { key }),
  clearGeminiKey: () => req<void>("DELETE", "/api/llm/keys/gemini"),
  /** Anthropic / Gemini / Ollama model catalogs, same shape so the
   *  Settings dropdown renders identically regardless of provider. */
  anthropicModels: () =>
    req<AnthropicModelsResponse>("GET", "/api/llm/anthropic/models"),
  geminiModels: () =>
    req<GeminiModelsResponse>("GET", "/api/llm/gemini/models"),
  ollamaModels: (url?: string) => {
    const path = url
      ? `/api/llm/ollama/models?url=${encodeURIComponent(url)}`
      : "/api/llm/ollama/models";
    return req<LlmModelEntry[]>("GET", path);
  },
  /// Per-file assistant conversation persistence. Each file's
  /// conversation is its own JSON under `.chan/assistant/`.
  /// `null` from get means no conversation yet (server returns
  /// 204).
  getConversation: async (path: string): Promise<unknown | null> => {
    const v = await req<unknown | undefined>(
      "GET",
      `/api/assistant/conversation?path=${encodeURIComponent(path)}`,
    );
    return v ?? null;
  },
  putConversation: (path: string, body: unknown) =>
    req<void>(
      "PUT",
      `/api/assistant/conversation?path=${encodeURIComponent(path)}`,
      body,
    ),
  deleteConversation: (path: string) =>
    req<void>(
      "DELETE",
      `/api/assistant/conversation?path=${encodeURIComponent(path)}`,
    ),
  /// Save a Q&A exchange under the configured answers_dir.
  /// Returns the relative path that was written.
  saveAnswer: (body: {
    prompt: string;
    answer: string;
    citations?: Array<{ path: string; heading?: string | null; snippet?: string | null }>;
  }) => req<{ path: string }>("POST", "/api/answers", body),
  /** Upload an image attachment. Multipart POST that the editor's `![`
   *  picker, drag-and-drop, and clipboard paste all funnel through.
   *  Returns the drive-relative path of the saved file (e.g.
   *  `attachments/2026-05-02-...png`). */
  uploadAttachment: async (file: File): Promise<{ path: string }> => {
    // Multipart upload skips the JSON-shaped request() helper because
    // FormData cannot be JSON-encoded; we hit fetch directly and
    // reuse the same auth token.
    const form = new FormData();
    form.append("file", file);
    const headers: Record<string, string> = {};
    const tk = transportAuthToken();
    if (tk) headers.authorization = `Bearer ${tk}`;
    const res = await fetch("/api/attachments", { method: "POST", headers, body: form });
    if (!res.ok) {
      const text = await res.text().catch(() => res.statusText);
      throw new ApiError(res.status, text || res.statusText);
    }
    return (await res.json()) as { path: string };
  },
  list: () => req<TreeEntry[]>("GET", "/api/files"),
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
  move: (from: string, to: string) => req<void>("POST", "/api/move", { from, to }),
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
  indexStatus: () => req<IndexStatus>("GET", "/api/index/status"),
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
  /// the browser frontend always uses "default" so a single drive
  /// has one session file. Returns `null` when none exists yet
  /// (server returns 204 → req() yields undefined → coerced to
  /// null for the caller's convenience).
  getSession: async (): Promise<unknown | null> => {
    const v = await req<unknown | undefined>("GET", "/api/session?w=default");
    return v ?? null;
  },
  /// Persist the session payload. Body shape is opaque to the
  /// server; the frontend sends `serializeLayout()` output.
  putSession: (body: unknown) =>
    req<void>("PUT", "/api/session?w=default", body),
  links: () => req<GraphSnapshot>("GET", "/api/links"),
  /// Typed graph payload powering the graph view tab.
  graph: () => req<GraphView>("GET", "/api/graph"),
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
