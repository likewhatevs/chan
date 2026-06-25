/// Runtime detection + Tauri event bridge for the launcher SPA.
///
/// The launcher ships as a browser SPA (served by chan-server over HTTP) and, on
/// the desktop loopback surface, as a chan-desktop Tauri webview. This module is
/// the desktop event bridge for backend events the plain-browser
/// surface never sees — today just `devserver-control-closed` (a connected
/// devserver's control terminal exited). It uses the GLOBAL Tauri API
/// (`window.__TAURI__`, exposed by `withGlobalTauri: true`); the launcher has no
/// `@tauri-apps/api` dependency, so there is nothing to import.

type UnlistenFn = () => void;

interface TauriEvent<T> {
  payload: T;
}

interface TauriEventApi {
  listen<T>(event: string, handler: (e: TauriEvent<T>) => void): Promise<UnlistenFn>;
}

type TauriWindow = Window &
  typeof globalThis & {
    __TAURI__?: { event?: TauriEventApi };
  };

/// True only when running inside chan-desktop's Tauri webview WITH the global
/// event API available. The launcher needs the event bridge specifically, so
/// this gates on `__TAURI__.event.listen` (a plain browser has neither).
export function hasTauriEvents(): boolean {
  return typeof (window as TauriWindow).__TAURI__?.event?.listen === "function";
}

/// Subscribe to a Tauri backend event, delivering its payload to `handler`.
/// Resolves to an unlisten handle (a no-op off-desktop). Best-effort: a missing
/// bridge or a failed `listen` degrades to a no-op so the launcher boot path
/// never breaks on a non-desktop surface.
export async function onTauriEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  const api = (window as TauriWindow).__TAURI__?.event;
  if (!api?.listen) return () => {};
  try {
    return await api.listen<T>(event, (e) => handler(e.payload));
  } catch (err) {
    console.warn(`onTauriEvent(${event}) failed:`, err);
    return () => {};
  }
}
