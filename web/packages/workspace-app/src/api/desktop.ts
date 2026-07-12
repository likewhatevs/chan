/// Runtime detection + Tauri IPC dispatch for chan-desktop.
///
/// chan ships as both a browser SPA (served by chan-server over
/// http) and a chan-desktop Tauri webview. Most UX is identical
/// across the two; this module is the bridge where surfaces
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
/// is fine -- it is gesture-permitted and shows no persistent button in
/// Chrome. Returns "" when the clipboard holds no text or the read fails
/// (the caller treats empty as "nothing to paste"). Note Cmd+V does NOT
/// use this: it rides xterm's native paste event (the user's own paste
/// gesture), which is buttonless everywhere -- this is only for the
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

/// Write clipboard text without needing a user gesture (the OSC 52 path).
///
/// An OSC 52 copy sequence arrives from the PTY with no user gesture, and
/// chan-desktop's WKWebView can reject a gesture-less
/// `navigator.clipboard.writeText()`. So on desktop we write it natively in
/// Rust through the `write_clipboard_text` IPC, which goes straight to the OS
/// clipboard and never needs a gesture. On web `navigator.clipboard.writeText()`
/// is the only option -- it is gesture-permitted in a foreground tab, which is
/// where the terminal lives. Best-effort: a failed native IPC logs and falls
/// back to the web API so the copy still has a chance to land.
export async function writeClipboardText(text: string): Promise<void> {
  if (isTauriDesktop()) {
    try {
      await tauriInvoke<void>("write_clipboard_text", { text });
      return;
    } catch (err) {
      console.warn("writeClipboardText: write_clipboard_text IPC failed", err);
    }
  }
  await navigator.clipboard?.writeText(text);
}

/// Write a PNG image onto the OS clipboard on desktop (`cs copy` of an
/// image). chan-desktop's WKWebView rejects a gesture-less
/// `navigator.clipboard.write()`, and `cs copy` arrives from a control
/// command with no user gesture, so on desktop we hand the PNG bytes to the
/// native `write_clipboard_image` IPC (arboard). Only meaningful on desktop;
/// the web path uses `navigator.clipboard.write` directly in the caller.
export async function writeClipboardImage(pngBytes: Uint8Array): Promise<void> {
  // Tauri serializes a Uint8Array to a number[] for a Vec<u8> arg.
  await tauriInvoke<void>("write_clipboard_image", { bytes: Array.from(pngBytes) });
}

/// Read a PNG image off the OS clipboard on desktop (`cs paste` of an image),
/// as raw PNG bytes. Returns `null` when the clipboard holds no image. Native
/// arboard read (`read_clipboard_image`) so it never trips WKWebView's paste
/// button, mirroring `readClipboardText`.
export async function readClipboardImage(): Promise<Uint8Array | null> {
  const bytes = await tauriInvoke<number[] | null>("read_clipboard_image");
  return bytes ? new Uint8Array(bytes) : null;
}

/// Write HTML (with a plain-text fallback) onto the OS clipboard on desktop
/// (`cs copy --html`). Native arboard HTML setter so a real browser reading
/// the OS clipboard (a paste into Gmail) keeps the formatting.
export async function writeClipboardHtml(html: string, altText: string): Promise<void> {
  await tauriInvoke<void>("write_clipboard_html", { html, altText });
}

/// Read HTML off the OS clipboard on desktop (`cs paste --html`). Returns
/// `null` when the clipboard holds no HTML. Native arboard read.
export async function readClipboardHtml(): Promise<string | null> {
  return await tauriInvoke<string | null>("read_clipboard_html");
}

/// Absolute paths of the OS files currently on the macOS drag
/// pasteboard (the `read_dropped_paths` IPC). Only meaningful when
/// called from inside a `drop` event handler -- the drag pasteboard
/// persists until the next drag starts. Returns `[]` in a plain
/// browser, on non-macOS desktops, when the pasteboard holds no file
/// items, or when the ACL refuses the command (remote-served window
/// kinds don't get it) -- every failure degrades to a silent no-op so
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

/// One file chosen in the native upload picker: base name + raw bytes (the
/// `Vec<u8>` crosses the IPC bridge as a JSON number array).
export interface PickedUploadFile {
  name: string;
  bytes: number[];
}

/// Raise chan-desktop's NATIVE multi-file open picker (the `pick_upload_files`
/// IPC) and return the chosen files' bytes. `cs upload` cannot raise the SPA's
/// programmatic `<input type=file>` click on WKWebView (no user gesture, so the
/// click is silently dropped -- the same wall as the clipboard-paste quirk), so
/// on desktop we open a native panel in Rust instead; the caller wraps the
/// results in `File` objects and feeds the same upload pipeline the Inspector
/// pill uses. `[]` = the user cancelled. THROWS on ACL refusal (remote-served
/// `outbound-*` windows don't get it) or IPC failure -- an explicit `cs upload`
/// deserves a visible error, not a silent no-op (which is the bug we're fixing).
export async function pickUploadFiles(): Promise<PickedUploadFile[]> {
  return tauriInvoke<PickedUploadFile[]>("pick_upload_files");
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

/// Bury (hide, don't destroy) THIS workspace window: the Hide answer to the
/// desktop close-confirm overlay. The OS red-dot already fired and the host
/// prevented the close and asked the SPA; picking Hide invokes the
/// `hide_window_from_close_confirm` command, which buries the window (its
/// sessions stay warm, reopenable from the Window menu) instead of destroying
/// it. No-op off-desktop (the overlay is desktop-only by construction). INERT if
/// the ACL withholds the command (best-effort: a failed IPC logs and leaves the
/// window as-is rather than throwing into the overlay).
export async function hideWindowFromCloseConfirm(): Promise<void> {
  if (!isTauriDesktop()) return;
  try {
    await tauriInvoke("hide_window_from_close_confirm");
  } catch (err) {
    console.warn(
      "hideWindowFromCloseConfirm: hide_window_from_close_confirm IPC failed",
      err,
    );
  }
}

/// Abandon the devserver backing THIS workspace window: tell the desktop to
/// disconnect it. Called by the disconnect overlay's Abandon button on a
/// devserver-backed desktop window whose watcher channel has dropped. The desktop
/// resolves which devserver this window belongs to (from its window label), shows
/// the launcher, and drives the disconnect; the window then closes async via the
/// watcher. Throws on IPC/ACL failure so the reconnect overlay can surface a
/// visible error instead of leaving the action silent.
export async function abandonDevserverForWindow(): Promise<void> {
  if (!isTauriDesktop()) throw new Error("not running under Tauri");
  try {
    await tauriInvoke("abandon_devserver_for_window");
  } catch (err) {
    console.warn(
      "abandonDevserverForWindow: abandon_devserver_for_window IPC failed",
      err,
    );
    throw err;
  }
}

/// Reconnect the devserver backing THIS workspace window: tell the desktop to
/// force-close the (dead) control terminal and re-run the connect flow. Called
/// by the disconnect overlay's Reconnect button on a devserver-backed desktop
/// window. The desktop resolves which devserver this window belongs to (from
/// its window label). Throws on IPC/ACL failure so the reconnect overlay can
/// surface a visible error instead of leaving the action silent.
export async function reconnectDevserverForWindow(): Promise<void> {
  if (!isTauriDesktop()) throw new Error("not running under Tauri");
  try {
    await tauriInvoke("reconnect_devserver_for_window");
  } catch (err) {
    console.warn(
      "reconnectDevserverForWindow: reconnect_devserver_for_window IPC failed",
      err,
    );
    throw err;
  }
}

/// Drive the chan-desktop native window in or out of fullscreen. WKWebView
/// on macOS disables the HTML element Fullscreen API (`element.requestFullscreen()`
/// rejects), so the slide player's "play" mode cannot go fullscreen through the
/// DOM there. Instead it drives the native window through Tauri's built-in
/// `core:window` `set_fullscreen` command (the `plugin:window|set_fullscreen`
/// channel, `value` = the desired state, no `label` so it targets the calling
/// window). The `workspace` capability grants `core:window:allow-set-fullscreen`.
/// No-op off-desktop: the browser slide player keeps `element.requestFullscreen()`.
/// Best-effort: a failed IPC (e.g. a tunnel-origin devserver window whose remote
/// ACL withholds the command) logs and leaves the window as-is, so the player
/// still opens in-window rather than throwing into the slide keymap.
export async function setWindowFullscreen(on: boolean): Promise<void> {
  if (!isTauriDesktop()) return;
  try {
    await tauriInvoke("plugin:window|set_fullscreen", { value: on });
  } catch (err) {
    console.warn("setWindowFullscreen: set_fullscreen IPC failed", err);
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
