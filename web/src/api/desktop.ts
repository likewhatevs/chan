/// Runtime detection + Tauri IPC dispatch for chan-desktop.
///
/// chan ships as both a browser SPA (served by chan-server over
/// http) and a chan-desktop Tauri webview. Most UX is identical
/// across the two; this module is the small seam where surfaces
/// need to dispatch differently (window reload, DevTools, etc.).

import {
  beginTransfer,
  cancelTransfer,
  failTransfer,
  finishTransfer,
  setTransferProgress,
} from "../state/transfers.svelte";

type TauriWindow = Window &
  typeof globalThis & {
    __TAURI_INTERNALS__?: {
      invoke?: (cmd: string, args?: unknown) => Promise<unknown>;
    };
    __TAURI__?: {
      invoke?: (cmd: string, args?: unknown) => Promise<unknown>;
      core?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> };
    };
  };

/// True when running inside chan-desktop's Tauri webview.
///
/// Tauri 2 injects `window.__TAURI_INTERNALS__`; older Tauri
/// versions injected `window.__TAURI__`. Either global means
/// the SPA is hosted by a Tauri webview.
export function isTauriDesktop(): boolean {
  const w = window as TauriWindow;
  return Boolean(w.__TAURI__ || w.__TAURI_INTERNALS__);
}

/// Host OS of the chan-desktop shell, resolved once and cached.
///
/// `null` means "not yet resolved" or "not on desktop". We ask the
/// Rust side (`platform_os`, which returns `std::env::consts::OS`)
/// rather than sniffing `navigator.userAgent`: the compiled-in
/// target triple is exact, whereas a webview UA string can be
/// patched or spoofed and is awkward to map to a clean OS token.
/// Resolved values: "macos" | "linux" | "windows" | other.
let cachedDesktopOs: string | null = null;

/// Resolve (and cache) the desktop host OS. Returns `null` on web
/// (no Tauri) or if the IPC fails. Callers that need the value at
/// mount time should `await` this once and store the result; the
/// cache makes repeat calls cheap.
export async function desktopOs(): Promise<string | null> {
  if (!isTauriDesktop()) return null;
  if (cachedDesktopOs) return cachedDesktopOs;
  try {
    const os = await tauriInvoke<string>("platform_os");
    cachedDesktopOs = os;
    return os;
  } catch (err) {
    console.warn("desktopOs: platform_os IPC failed", err);
    return null;
  }
}

/// True only inside chan-desktop running on macOS. The native PDF
/// export path (WKWebView `createPDF`) exists only on macOS; web
/// keeps `window.print()` and other desktop OSes hide the button.
export async function isMacDesktop(): Promise<boolean> {
  return (await desktopOs()) === "macos";
}

/// Thin wrapper over Tauri's invoke. Resolves whichever invoke
/// shape the running Tauri version exposes. Throws when called
/// outside a Tauri webview so callers can branch on
/// `isTauriDesktop()` before dispatching.
export async function tauriInvoke<T = unknown>(
  cmd: string,
  args?: unknown,
): Promise<T> {
  const w = window as TauriWindow;
  const invoke =
    w.__TAURI_INTERNALS__?.invoke ??
    w.__TAURI__?.core?.invoke ??
    w.__TAURI__?.invoke;
  if (!invoke) throw new Error(`tauriInvoke(${cmd}): not running under Tauri`);
  return (await invoke(cmd, args)) as T;
}

/// Read clipboard text without tripping WebKit's DOM-paste "Paste" button.
///
/// In chan-desktop's WKWebView, any programmatic clipboard read via
/// `navigator.clipboard.readText()` pops a native "Paste" permission
/// button the user must click (a WebKit privacy feature with no JS
/// opt-out). So on desktop we read it natively in Rust through the
/// `read_clipboard_text` IPC, which goes straight to the OS clipboard
/// and never shows the button. On web `navigator.clipboard.readText()`
/// is fine — it is gesture-permitted and shows no persistent button in
/// Chrome. Returns "" when the clipboard holds no text or the read fails
/// (the caller treats empty as "nothing to paste"). Note Cmd+V does NOT
/// use this: it rides xterm's native paste event (the user's own paste
/// gesture), which is buttonless everywhere — this is only for the
/// right-click menu's "Paste", where no paste gesture exists.
export async function readClipboardText(): Promise<string> {
  if (isTauriDesktop()) {
    try {
      return await tauriInvoke<string>("read_clipboard_text");
    } catch (err) {
      console.warn("readClipboardText: read_clipboard_text IPC failed", err);
    }
  }
  return (await navigator.clipboard?.readText()) ?? "";
}

/// Absolute paths of the OS files currently on the macOS drag
/// pasteboard (the `read_dropped_paths` IPC). Only meaningful when
/// called from inside a `drop` event handler — the drag pasteboard
/// persists until the next drag starts. Returns `[]` in a plain
/// browser, on non-macOS desktops, when the pasteboard holds no file
/// items, or when the ACL refuses the command (remote-served window
/// kinds don't get it) — every failure degrades to a silent no-op so
/// the drop guard's no-takeover guarantee is all that remains.
export async function readDroppedPaths(): Promise<string[]> {
  if (!isTauriDesktop()) return [];
  try {
    return await tauriInvoke<string[]>("read_dropped_paths");
  } catch {
    // ACL refusal (tunnel/outbound windows) or pre-IPC desktop build.
    return [];
  }
}

/// Reload the chan window. On chan-desktop calls the
/// `reload_window` IPC which fires `WebviewWindow::reload()`.
/// Falls back to `window.location.reload()` on web or on IPC
/// failure so the user still gets the reload they asked for.
export async function reloadWindow(): Promise<void> {
  if (isTauriDesktop()) {
    try {
      await tauriInvoke("reload_window");
      return;
    } catch (err) {
      console.warn(
        "reloadWindow: reload_window IPC failed, falling back",
        err,
      );
    }
  }
  window.location.reload();
}

/// Close the current workspace window and return focus to the
/// launcher (the native-desktop workspace list). Called when the
/// last tab and then the last empty pane are closed. No-op
/// off-desktop: the browser owns its own window/tab lifecycle.
/// Best-effort: a failed IPC logs and leaves the window as-is
/// rather than throwing into the keymap.
export async function requestCloseWindow(): Promise<void> {
  if (!isTauriDesktop()) return;
  try {
    await tauriInvoke("request_close_window");
  } catch (err) {
    console.warn("requestCloseWindow: request_close_window IPC failed", err);
  }
}

/// Open the platform's web inspector. On chan-desktop calls the
/// `open_devtools` IPC. On web returns false so the caller can
/// surface a hint pointing the user at the browser's built-in
/// DevTools.
export async function openWebInspector(): Promise<boolean> {
  if (!isTauriDesktop()) return false;
  try {
    await tauriInvoke("open_devtools");
    return true;
  } catch (err) {
    console.warn("openWebInspector: open_devtools IPC failed", err);
    return false;
  }
}

/// Desktop-native download for the inspector's Download button.
/// The browser hands `<a download>` to its own download manager;
/// the desktop webview has none, so this fetches the file over
/// the loopback connection with XHR progress (shown in the transfer
/// bubble) and writes it to the user's Downloads folder via the
/// `save_file_to_downloads` Tauri command.
///
/// `url` is the absolute tokenized download URL (the caller computes it
/// from `api.downloadUrl(path)` resolved against `window.location`).
/// `filename` is the suggested name (the FileTree `downloadFilename`).
/// Resolves to the saved absolute path on success; rejects on error.
/// The store carries progress / cancel / savedPath / error so the
/// inspector can render the indicator without re-deriving anything.
export async function runDesktopDownload(
  url: string,
  filename: string,
  source: { path: string; isDir: boolean } | null = null,
): Promise<string> {
  if (!isTauriDesktop()) {
    throw new Error("runDesktopDownload called outside chan-desktop");
  }
  const xhr = new XMLHttpRequest();
  // The transfer bubble is the single download surface. `source` lets an
  // interrupted download offer Retry after a window reload.
  const xferId = beginTransfer({
    kind: "download",
    filename,
    cancel: () => xhr.abort(),
    source,
  });
  try {
    const bytes = await new Promise<Uint8Array>((resolve, reject) => {
      xhr.open("GET", url);
      xhr.responseType = "arraybuffer";
      xhr.onprogress = (event) => {
        // Content-Length present -> a real ratio; otherwise leave the
        // indicator indeterminate (null) rather than faking a number.
        const frac =
          event.lengthComputable && event.total > 0 ? event.loaded / event.total : null;
        setTransferProgress(xferId, frac);
      };
      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          resolve(new Uint8Array(xhr.response as ArrayBuffer));
        } else {
          reject(new Error(`download failed: HTTP ${xhr.status}`));
        }
      };
      xhr.onerror = () => reject(new Error("download failed: network error"));
      xhr.onabort = () => reject(new Error("download cancelled"));
      xhr.send();
    });
    // Past this point cancelling no longer applies (the bytes are in
    // hand); the store clears its cancel on finish/fail.
    const saved = await tauriInvoke<{ path: string }>(
      "save_file_to_downloads",
      // Tauri serializes a Uint8Array to a number[] for a Vec<u8> arg.
      { filename, bytes: Array.from(bytes) },
    );
    finishTransfer(xferId, saved.path);
    return saved.path;
  } catch (err) {
    const message = (err as Error)?.message ?? String(err);
    // A user abort is a cancel, not a failure (the bubble shows "Cancelled").
    if (message === "download cancelled") cancelTransfer(xferId);
    else failTransfer(xferId, message);
    throw err instanceof Error ? err : new Error(message);
  }
}

/// Save bytes the SPA already holds in memory to the Downloads folder
/// through the same `save_file_to_downloads` command + progress
/// indicator that `runDesktopDownload` uses. Unlike that path there is
/// no network fetch (the bytes are produced locally, e.g. the native PDF
/// export), so the transfer goes straight from begin -> finish.
///
/// Resolves to the saved absolute path; rejects on error. Reuses the
/// existing capability, so no new Tauri permission is required.
export async function saveBytesToDownloads(
  bytes: Uint8Array,
  filename: string,
): Promise<string> {
  if (!isTauriDesktop()) {
    throw new Error("saveBytesToDownloads called outside chan-desktop");
  }
  // The bytes already exist, so cancelling has nothing to abort; pass a null
  // cancel and let the bubble show an indeterminate-then-done transfer.
  const xferId = beginTransfer({ kind: "download", filename, cancel: null });
  try {
    const saved = await tauriInvoke<{ path: string }>("save_file_to_downloads", {
      // Tauri serializes a Uint8Array to a number[] for a Vec<u8> arg.
      filename,
      bytes: Array.from(bytes),
    });
    finishTransfer(xferId, saved.path);
    return saved.path;
  } catch (err) {
    const message = (err as Error)?.message ?? String(err);
    failTransfer(xferId, message);
    throw err instanceof Error ? err : new Error(message);
  }
}
