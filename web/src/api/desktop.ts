/// Runtime detection + Tauri IPC dispatch for chan-desktop.
///
/// chan ships as both a browser SPA (served by chan-server over
/// http) and a chan-desktop Tauri webview. Most UX is identical
/// across the two; this module is the small seam where surfaces
/// need to dispatch differently (window reload, DevTools, etc.).

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
