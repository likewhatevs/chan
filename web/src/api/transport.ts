// API transport: HTTP+WebSocket against the local chan-server.
//
// `chan serve` runs a real loopback server. The Tauri desktop and
// (eventual) iOS shells spawn the same server in-process and point
// their WebView at it via the same loopback URL. One transport
// implementation, one wire format, one auth model - there is no
// platform-specific path.
//
// The seam exists as a single module so client.ts can stay focused
// on the typed API surface (file ops, search, graph, ...) and leave
// wire mechanics (token plumbing, fetch shape, WebSocket reconnect)
// here.
//
// We considered routing native shells through Tauri's custom URI
// scheme + event bus to avoid binding a port. That doesn't work as
// cleanly as it sounds: HTTP request/response would map fine, but
// WebSocket can't go through a custom scheme (the browser spec
// requires `ws:` / `wss:` URLs) and Tauri events are pub/sub with
// none of WebSocket's ordering or connection-state semantics.
// Splitting just HTTP over a custom scheme while leaving WS on
// loopback was also evaluated: since we'd still bind a port for
// /ws, the partial scheme switch added complexity without removing
// the open port. We stay on plain loopback everywhere.

import { ApiError } from "./errors";
import type { WsClientFrame } from "./types";

export type WsStatus = "connecting" | "open" | "reconnecting" | "closed";

/// Handle for the watcher socket. `send` pushes a client -> server frame
/// (the scope sub/unsub path); it is best-effort: a frame queued while the
/// socket is connecting or reconnecting is dropped, so subscription state is
/// re-established on reconnect by the owner, not buffered here. `close`
/// closes the socket and stops reconnecting. The handle is also directly
/// callable as the disposer for backward compatibility with the original
/// `() => void` return.
export interface WatchSocket {
  (): void;
  send(frame: WsClientFrame): void;
  close(): void;
}

const TOKEN_KEY = "chan.token";

function readPrefix(): string {
  const m = document.querySelector('meta[name="chan-prefix"]');
  const v = m?.getAttribute("content")?.trim() ?? "";
  // The server only injects the tag when a prefix is set, but be
  // defensive: a stray empty / non-canonical value collapses to "".
  if (!v || !v.startsWith("/")) return "";
  return v.replace(/\/+$/, "");
}

/// True when the server told the SPA shell to lock down the
/// Settings entry point (any tunnel mode run). Read once from the
/// `<meta name="chan-settings-disabled">` tag. The UI greys out the
/// Settings button; the matching write-side server routes refuse
/// with 403 so a curl bypass can't sidestep the lock.
export const SETTINGS_DISABLED = readBoolMeta("chan-settings-disabled");

function readBoolMeta(name: string): boolean {
  const m = document.querySelector(`meta[name="${name}"]`);
  return m?.getAttribute("content")?.trim() === "1";
}

/// Server URL prefix when `chan serve --prefix=/foo` mounts the
/// API under a path. Read once at module load from the
/// `<meta name="chan-prefix">` tag the server injects into the SPA
/// shell. Empty string when no prefix.
const PREFIX = readPrefix();

/// Prepend the server URL prefix to an in-app path. Pass paths with
/// a leading slash (`/api/foo`); the result is the absolute path the
/// browser should fetch. Used by `request`, `withTokenQuery`, and
/// any direct `fetch` outside the request helper (multipart upload,
/// `<img>` URLs).
export function apiPath(path: string): string {
  if (!PREFIX) return path;
  if (!path.startsWith("/")) return `${PREFIX}/${path}`;
  return `${PREFIX}${path}`;
}

function loadToken(): string | null {
  const url = new URL(window.location.href);
  const t = url.searchParams.get("t");
  if (t) {
    sessionStorage.setItem(TOKEN_KEY, t);
    url.searchParams.delete("t");
    window.history.replaceState({}, "", url.toString());
    return t;
  }
  return sessionStorage.getItem(TOKEN_KEY);
}

const token = loadToken();

/// Append the auth token as a `?t=...` query and apply the server
/// URL prefix. Use only for paths that can't carry an Authorization
/// header (WebSocket upgrade, `<img src>` rendered by the browser).
export function withTokenQuery(path: string): string {
  const full = apiPath(path);
  if (!token) return full;
  const sep = full.includes("?") ? "&" : "?";
  return `${full}${sep}t=${encodeURIComponent(token)}`;
}

/// Raw auth token. Exposed for the few call sites that build URLs
/// outside the request helper (image src, multipart upload).
/// Returns null on a `--no-token` server.
export function authToken(): string | null {
  return token;
}

/// Issue a JSON-shaped request. Returns parsed JSON, or undefined
/// for 204 / empty responses. Throws ApiError on non-2xx, or on
/// the wall-clock timeout (10 s by default) so the UI never
/// deadlocks behind a hung fetch.
///
/// The timeout matters specifically on iOS WKWebView: the first
/// connect attempt to loopback can occasionally stall indefinitely
/// (we've seen "Network is down" recoverable errors that the
/// underlying NSURLSession doesn't always surface back to fetch).
/// Without an upper bound, restoreLayout's `await
/// loadTabContent(...)` for a stuck tab would block bootstrap
/// forever and the user would see "loading..." with no way out.
///
/// Pass `timeoutMs: 0` to disable the cap. Reserve that for
/// endpoints that legitimately run for minutes and rely on the
/// caller-supplied AbortSignal for user-initiated cancel.
export async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
  timeoutMs: number = REQUEST_TIMEOUT_MS,
): Promise<T> {
  const headers: Record<string, string> = {};
  if (token) headers.authorization = `Bearer ${token}`;
  const init: RequestInit = { method, headers };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
    headers["content-type"] = "application/json";
  }

  // Compose the caller's signal (if any) with our timeout signal so
  // either an explicit caller abort or the timeout cancels the
  // fetch. AbortSignal.any was added in Safari 17.4 / Chrome 124;
  // both are below the WebViews chan targets.
  const timeoutCtl = new AbortController();
  const timer =
    timeoutMs > 0 ? setTimeout(() => timeoutCtl.abort(), timeoutMs) : null;
  const sigs: AbortSignal[] = [];
  if (timer !== null) sigs.push(timeoutCtl.signal);
  if (signal) sigs.push(signal);
  if (sigs.length > 0) init.signal = AbortSignal.any(sigs);

  try {
    const res = await fetch(apiPath(path), init);
    if (!res.ok) {
      const text = await res.text().catch(() => res.statusText);
      // Try to parse the body as JSON so structured error responses
      // (the 409 { current_mtime_ns } conflict body, the standard
      // { error } wrapper) reach the caller as ApiError.data. Any
      // non-JSON body falls back to the textual message.
      let data: unknown = null;
      let message = text || res.statusText;
      if (text) {
        try {
          data = JSON.parse(text);
          if (
            data &&
            typeof data === "object" &&
            "error" in (data as Record<string, unknown>) &&
            typeof (data as { error: unknown }).error === "string"
          ) {
            message = (data as { error: string }).error;
          }
        } catch {
          // Not JSON; keep the raw text as the message.
        }
      }
      throw new ApiError(res.status, message, data);
    }
    const text = await res.text();
    if (!text) return undefined as T;
    return JSON.parse(text) as T;
  } catch (e) {
    // Differentiate between the caller's abort, our timeout, and
    // any other fetch error. The caller's AbortError stays opaque;
    // our own timeout becomes a dedicated message so the UI can
    // surface "request timed out" instead of "AbortError".
    if (timeoutCtl.signal.aborted && (signal === undefined || !signal.aborted)) {
      throw new ApiError(0, `request timed out after ${timeoutMs} ms: ${method} ${path}`);
    }
    throw e;
  } finally {
    if (timer !== null) clearTimeout(timer);
  }
}

/// Default wall-clock cap on a single request. Generous so
/// legitimately slow operations (large file read, model download
/// on first search) still complete, tight enough that a hung
/// fetch on loopback fails the UI fast instead of locking it up.
/// Callers that need a different cap pass `timeoutMs` to
/// `request`; `0` disables.
const REQUEST_TIMEOUT_MS = 10_000;

/// Open the watcher subscription. Auto-reconnects with capped
/// exponential backoff (500 ms ramping to 8 s). The status callback
/// fires on connect/reconnect transitions so the UI can show the
/// disconnect overlay when the channel drops. Returns a disposer
/// that closes the socket and stops reconnecting.
///
/// The disposer detaches every WS event handler before calling
/// `close()`. Without that, `reconnectWatcher` (which calls the
/// disposer and immediately opens a fresh socket) would race: the
/// old socket's async `onclose` fires after the new socket has
/// already pushed `"connecting"` to the status callback, and the
/// stale handler would clobber it back to a disconnected state.
/// This was the cause of the "Retry now" button appearing to do
/// nothing on iOS lock/unlock: the new socket WAS connecting, the
/// old socket's onclose was just stomping the status afterwards.
export function openWatch(
  onEvent: (e: unknown) => void,
  onStatus: (s: WsStatus) => void = () => {},
  onOpen: () => void = () => {},
  windowId?: string,
): WatchSocket {
  let closed = false;
  let ws: WebSocket | null = null;
  let backoff = 500;

  const connect = () => {
    if (closed) return;
    onStatus("connecting");
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    // withTokenQuery applies the server prefix and the ?t= token to
    // the path; the caller stitches on proto+host to produce the
    // absolute WS URL. `w=<windowId>` tags the socket with this
    // window's session id so the server's WindowPresence (and thus
    // GET /api/windows + `cs window list`) knows the window is
    // connected. The caller supplies the id (client.ts owns the
    // sessionWindowId logic; importing it here would cycle).
    let path = withTokenQuery("/ws");
    if (windowId) {
      const sep = path.includes("?") ? "&" : "?";
      path = `${path}${sep}w=${encodeURIComponent(windowId)}`;
    }
    const url = `${proto}//${window.location.host}${path}`;
    ws = new WebSocket(url);
    ws.onopen = () => {
      backoff = 500;
      onStatus("open");
      // The server's scope registry is per-socket, so a fresh socket
      // starts with no subscriptions. The owner re-establishes its
      // active scopes here (the File Browser instances wire that in);
      // the transport does not buffer pre-open frames.
      onOpen();
    };
    ws.onmessage = (m) => {
      try {
        onEvent(JSON.parse(m.data));
      } catch {
        // Drop malformed frames; the server controls the wire format.
      }
    };
    ws.onclose = () => {
      if (closed) return;
      onStatus("reconnecting");
      const delay = backoff;
      backoff = Math.min(backoff * 2, 8000);
      setTimeout(connect, delay);
    };
  };

  const close = () => {
    closed = true;
    const w = ws;
    ws = null;
    if (w) {
      // Defuse the handlers BEFORE close() so a queued `onclose`
      // event doesn't fire after the next `connect()` already set
      // the status to "connecting".
      w.onopen = null;
      w.onclose = null;
      w.onerror = null;
      w.onmessage = null;
      try {
        w.close();
      } catch {
        // close() can throw if the socket is already in CLOSED
        // state; that's exactly what we wanted, swallow it.
      }
    }
  };

  const send = (frame: WsClientFrame) => {
    // Best-effort: only push on an OPEN socket. A sub/unsub queued
    // while connecting/reconnecting is dropped on purpose; the owner
    // re-subscribes from `onOpen` after the reconnect, so buffering
    // here would risk replaying stale subscription intent.
    if (ws && ws.readyState === WebSocket.OPEN) {
      try {
        ws.send(JSON.stringify(frame));
      } catch {
        // A send can throw if the socket flipped to CLOSING between
        // the readyState check and here; the reconnect path will
        // re-establish subscriptions.
      }
    }
  };

  connect();

  // Callable disposer (back-compat with the original `() => void`
  // return) plus the typed scope-control methods.
  const handle = (() => close()) as WatchSocket;
  handle.send = send;
  handle.close = close;
  return handle;
}

/// Wire frame for the per-library focus-colour watch
/// (`GET /api/library/local-color/watch`): `{ color }`, a hex string or null,
/// mirroring the GET/PUT `LocalColor` shape. Pushed on connect + on each change.
interface LocalColorFrame {
  color: string | null;
}

/// Open the per-library focus-colour watch (Theme 6). A dedicated, self-contained
/// WebSocket to `/api/library/local-color/watch` (bearer via `?t=`) that calls
/// `onColor` with each pushed colour (hex or null) — push-on-connect + on change.
/// Auto-reconnects with the same capped backoff as `openWatch` (500 ms → 8 s).
///
/// Deliberately NOT built on the `/ws` watcher: this channel carries none of its
/// windowId/scope machinery, and keeping it separate leaves the load-bearing
/// window watcher untouched. Best-effort — no status callback (it never drives a
/// disconnect overlay). Returns a disposer that closes the socket and stops
/// reconnecting; the disposer defuses the handlers before `close()` so a queued
/// `onclose` can't schedule a reconnect after disposal.
export function openLocalColorWatch(
  onColor: (color: string | null) => void,
): () => void {
  let closed = false;
  let ws: WebSocket | null = null;
  let backoff = 500;

  const connect = () => {
    if (closed) return;
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const path = withTokenQuery("/api/library/local-color/watch");
    ws = new WebSocket(`${proto}//${window.location.host}${path}`);
    ws.onopen = () => {
      backoff = 500;
    };
    ws.onmessage = (m) => {
      try {
        const frame = JSON.parse(m.data) as LocalColorFrame;
        onColor(frame?.color ?? null);
      } catch {
        // Drop malformed frames; the server controls the wire format.
      }
    };
    ws.onclose = () => {
      if (closed) return;
      const delay = backoff;
      backoff = Math.min(backoff * 2, 8000);
      setTimeout(connect, delay);
    };
  };

  const close = () => {
    closed = true;
    const w = ws;
    ws = null;
    if (w) {
      w.onopen = null;
      w.onclose = null;
      w.onerror = null;
      w.onmessage = null;
      try {
        w.close();
      } catch {
        // close() can throw if the socket is already CLOSED — the desired end
        // state, so swallow it.
      }
    }
  };

  connect();
  return close;
}
