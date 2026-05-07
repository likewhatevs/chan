// Tauri-native helpers. Anything that talks to the desktop or
// mobile shell goes through here so the rest of the frontend can
// keep treating the app as a plain web client.
//
// Native vs browser detection: Tauri 2 exposes
// `window.__TAURI_INTERNALS__` as soon as a Tauri WebView mounts;
// `chan serve` running in a normal browser leaves that undefined.

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const native =
  typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;

/// Mobile platform marker. chan-app's mobile boot path appends
/// `?platform=ios` (or eventually `android`) to the loopback URL so
/// the frontend can swap Toolbar+Workspace for MobileShell. We
/// don't sniff the user agent: the chan-app target_os is the
/// source of truth, and the WKWebView's UA on iOS is desktop-Safari
/// shaped anyway.
const platform = (() => {
  if (typeof window === "undefined") return null;
  return new URL(window.location.href).searchParams.get("platform");
})();

export function isNativeDesktop(): boolean {
  return native && platform !== "ios" && platform !== "android";
}

/// True when the app is running inside chan-app's iOS or Android
/// shell. The desktop chan-app and chan serve both return false.
export function isMobile(): boolean {
  return platform === "ios" || platform === "android";
}

/// True for iPad (and large-screen Android) running the mobile build:
/// same chan-app target as a phone, but the viewport is wide enough
/// to host a split-pane layout. Threshold matches Apple's "regular"
/// vs "compact" horizontal size class boundary (768 pt). iPhone in
/// landscape stays under that threshold and therefore single-pane.
export function isTablet(): boolean {
  if (!isMobile()) return false;
  if (typeof window === "undefined") return false;
  return window.innerWidth >= 768;
}

/// Specific platform string for fine-grained branching where
/// iOS-only / Android-only behaviour matters (storage paths,
/// keyboard accessory bars, etc.). Null on every non-mobile build.
export function mobilePlatform(): "ios" | "android" | null {
  if (platform === "ios") return "ios";
  if (platform === "android") return "android";
  return null;
}

/// Stable per-window identifier for session keying. Tauri's
/// chan-app passes its own window label via `?w=<id>`; chan serve
/// in the browser falls back to "default" so a single tab hits one
/// shared session file. Computed once at module load and cached so
/// every call returns the same string for the lifetime of the
/// document.
const WINDOW_ID = (() => {
  if (typeof window === "undefined") return "default";
  const w = new URL(window.location.href).searchParams.get("w");
  if (!w) return "default";
  // Match the server's sanitize_window_id rules so we don't put a
  // value through the URL that the server would normalize. Empty
  // result -> "default" (server applies the same fallback).
  const cleaned = w.replace(/[^A-Za-z0-9_-]/g, "").slice(0, 64);
  return cleaned || "default";
})();

export function windowId(): string {
  return WINDOW_ID;
}

/// Read `?open=<path>` from the URL and strip it from the address
/// bar (so a refresh doesn't re-open the file). Older builds set
/// this when spawning a fresh workspace window in response to a
/// file open dispatched from a now-removed special-tab window;
/// kept so a stale URL hash from those builds still does the right
/// thing on first launch.
export function readAndConsumeOpenFile(): string | null {
  if (typeof window === "undefined") return null;
  const url = new URL(window.location.href);
  const p = url.searchParams.get("open");
  if (!p) return null;
  url.searchParams.delete("open");
  window.history.replaceState({}, "", url.toString());
  return p;
}

/// Subscribe to "chan:open-settings" events emitted by the Rust
/// menu handler when the user picks Settings... from the App
/// submenu (or hits Cmd+,). Same window-scoped listen pattern as
/// listenOpenFile so the emit only fires the focused window's
/// callback, not every open window's.
export async function listenOpenSettings(
  cb: () => void,
): Promise<() => void> {
  if (!native) return () => {};
  const { getCurrentWebviewWindow } = await import(
    "@tauri-apps/api/webviewWindow"
  );
  const win = getCurrentWebviewWindow();
  const unlisten = await win.listen<null>("chan:open-settings", () => {
    cb();
  });
  return unlisten;
}

/// Identifiers for menu-driven actions that the focused window
/// must handle (open Files window, open Search overlay, etc.).
/// Kept narrow on purpose: only items that benefit from being on
/// the macOS menubar AND need a window-scoped reaction.
export type MenuAction = "files" | "search" | "graph" | "assistant";

/// Subscribe to "chan:menu-action" events emitted by the Rust
/// menu handler with a string payload identifying which item the
/// user picked. Window-scoped listen pattern (see listenOpenFile).
export async function listenMenuAction(
  cb: (action: MenuAction) => void,
): Promise<() => void> {
  if (!native) return () => {};
  const { getCurrentWebviewWindow } = await import(
    "@tauri-apps/api/webviewWindow"
  );
  const win = getCurrentWebviewWindow();
  const unlisten = await win.listen<MenuAction>(
    "chan:menu-action",
    (e) => {
      cb(e.payload);
    },
  );
  return unlisten;
}

