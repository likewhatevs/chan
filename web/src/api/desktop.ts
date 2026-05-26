/// Runtime detection + Tauri IPC dispatch for chan-desktop.
///
/// chan ships as both a browser SPA (served by chan-server over
/// http) and a chan-desktop Tauri webview. Most UX is identical
/// across the two; this module is the small seam where surfaces
/// need to dispatch differently (window reload, DevTools, etc.).

import {
  beginDownloadTransfer,
  failDownloadTransfer,
  finishDownloadTransfer,
  setDownloadProgress,
} from "../state/downloadTransfer.svelte";

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

/// Reload the chan window. On chan-desktop calls the
/// `reload_window` IPC (see `fullstack-b-17`) which fires
/// `WebviewWindow::reload()`. On web (or if the IPC call fails)
/// falls back to `window.location.reload()` so the user still
/// gets the reload they asked for.
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

/// Open the platform's web inspector. On chan-desktop calls the
/// `open_devtools` IPC (see `fullstack-b-17`). On web returns
/// false so the caller can surface a hint pointing the user at
/// the browser's built-in DevTools.
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

/// Bug 2b: the desktop-native download capability the inspector's
/// Download button calls when running under chan-desktop. The browser
/// hands `<a download>` to its own download manager; the desktop
/// webview has none, so this fetches the file over the loopback
/// connection with XHR progress (driving the `downloadTransfer` store
/// for the in-app indicator) and writes it to the user's Downloads
/// folder via the `save_file_to_downloads` Tauri command.
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
): Promise<string> {
  if (!isTauriDesktop()) {
    throw new Error("runDesktopDownload called outside chan-desktop");
  }
  const xhr = new XMLHttpRequest();
  beginDownloadTransfer(filename, () => xhr.abort());
  try {
    const bytes = await new Promise<Uint8Array>((resolve, reject) => {
      xhr.open("GET", url);
      xhr.responseType = "arraybuffer";
      xhr.onprogress = (event) => {
        // Content-Length present -> a real ratio; otherwise leave the
        // indicator indeterminate (null) rather than faking a number.
        setDownloadProgress(
          event.lengthComputable && event.total > 0
            ? event.loaded / event.total
            : null,
        );
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
    finishDownloadTransfer(saved.path);
    return saved.path;
  } catch (err) {
    const message = (err as Error)?.message ?? String(err);
    failDownloadTransfer(message);
    throw err instanceof Error ? err : new Error(message);
  }
}
