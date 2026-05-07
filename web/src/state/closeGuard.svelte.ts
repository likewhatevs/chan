// Close-confirmation flow for the Tauri desktop app.
//
// When the user closes the app window with unsaved tab buffers, we
// intercept the close-requested event, ask whether to save before
// quitting, and act on their choice. Out of scope for the browser
// flow (`chan serve`): the browser tab is the user's own to manage,
// and we can't reliably block its unload anyway.
//
// Detection is via `__TAURI_INTERNALS__` which Tauri injects into
// the WebView. The Tauri JS API import is dynamic so the package
// stays out of the browser bundle's hot path until we know we
// need it.

import { layout, saveTab, type Tab } from "./tabs.svelte";

/// User's response to the close-confirmation modal.
/// - "save": flush every dirty buffer and then close the window.
///   On any save failure we surface the error and re-open the
///   modal so the user can retry, discard, or cancel.
/// - "discard": close the window without saving. Bypasses the
///   close-requested handler via the force_close Rust command.
///   Unsaved edits are lost.
/// - "cancel": leave the app open.
export type CloseGuardChoice = "save" | "discard" | "cancel";

/// State observed by `CloseGuardModal.svelte`. When `open` flips
/// to true, the modal renders and waits for the user to pick a
/// choice; the modal calls `resolveCloseGuard` to feed the
/// promise resumed by `setupCloseGuard`'s close handler.
export const closeGuardState = $state<{
  open: boolean;
  dirtyCount: number;
  /// Inline error from the previous save attempt, surfaced in the
  /// modal so the user can decide whether to retry, discard, or
  /// cancel. Cleared when the user resolves the modal.
  error: string | null;
  resolve: ((choice: CloseGuardChoice) => void) | null;
}>({
  open: false,
  dirtyCount: 0,
  error: null,
  resolve: null,
});

export function resolveCloseGuard(choice: CloseGuardChoice): void {
  const r = closeGuardState.resolve;
  closeGuardState.resolve = null;
  closeGuardState.open = false;
  closeGuardState.error = null;
  r?.(choice);
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/// Snapshot of tabs whose in-memory content differs from the last
/// saved value. Walked across every leaf pane in the active window.
function dirtyTabs(): Tab[] {
  const out: Tab[] = [];
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const t of node.tabs) {
      if (t.kind === "file" && t.content !== t.saved) out.push(t);
    }
  }
  return out;
}

function dirtyCount(): number {
  return dirtyTabs().length;
}

/// Wire the Tauri close-requested handler. Safe to call in non-
/// Tauri contexts (browser); becomes a no-op. Must be called once
/// at app boot (App.svelte's onMount).
export async function setupCloseGuard(): Promise<void> {
  if (!isTauri()) return;
  // Dynamic import so the @tauri-apps/api code never executes in
  // the browser (where the module would still load fine but spend
  // bytes on dead weight).
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const { getAllWebviewWindows } = await import(
    "@tauri-apps/api/webviewWindow"
  );
  const win = getCurrentWindow();
  const { invoke } = await import("@tauri-apps/api/core");
  // Force-close via a Rust command. The browser-facing
  // `WebviewWindow.destroy()` (`plugin:window|destroy`) is silently
  // a no-op when called from inside an onCloseRequested callback
  // that previously preventDefault'd the same event - the held
  // event keeps the window pinned in a "close prevented" state
  // and destroy() observes it. setTimeout(0) is not enough: the
  // suspension persists past microtask + macrotask boundaries.
  // The custom `force_close_window` command runs on the Rust side
  // outside any JS event scope, so window.destroy() actually
  // takes effect.
  const forceClose = (): void => {
    void invoke("force_close_window", { label: win.label });
  };
  await win.onCloseRequested(async (event) => {
    // Tauri 2 only honours preventDefault in the synchronous
    // prologue of the listener. We always cross at least one await
    // before deciding, so preventDefault upfront and route every
    // "close anyway" branch through forceClose().
    event.preventDefault();
    if (dirtyCount() === 0) {
      forceClose();
      return;
    }
    // Only prompt on the LAST workspace window. Earlier closes
    // proceed silently because the dirty buffers belong to other
    // open windows and will get their own prompt when those close.
    // Workspace windows are labelled "main" or "win-<N>"; special-
    // tab windows use "view-..." and skip setupCloseGuard entirely
    // (App.svelte gates on singleView), so they don't show up here.
    const all = await getAllWebviewWindows();
    const workspaces = all.filter((w) => {
      const l = w.label;
      return l === "main" || l.startsWith("win-");
    });
    if (workspaces.length > 1) {
      forceClose();
      return;
    }
    const choice = await prompt(dirtyCount());
    if (choice === "cancel") {
      return;
    }
    if (choice === "discard") {
      forceClose();
      return;
    }
    try {
      await Promise.all(dirtyTabs().map((t) => saveTab(t)));
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("save-on-quit failed:", e);
      closeGuardState.error = (e as Error).message ?? String(e);
      const retry = await prompt(dirtyCount());
      if (retry === "discard") {
        forceClose();
      }
      return;
    }
    forceClose();
  });
}

function prompt(n: number): Promise<CloseGuardChoice> {
  return new Promise((resolve) => {
    closeGuardState.dirtyCount = n;
    closeGuardState.resolve = resolve;
    closeGuardState.open = true;
  });
}
