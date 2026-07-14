// Net-new Global commands: theme (system / light / dark), the screen-lock
// family (enable / disable / test / set pin / theme), and the window
// controls (Reload, Open Inspector, Hide window) that mirror the WebView
// native menu. The reuse-existing Global entries live in core.ts; these
// need a new action or an in-app prompt. Theme, screen lock, and Reload
// are machine-global; Open Inspector and Hide window are desktop-only.
// Register with registerCommands. See state/commands.ts for the Command
// shape and helpers.

import { registerCommands } from "../commands";
import {
  setThemeChoice,
  setTransientStatus,
  uiPrompt,
  workspace,
} from "../store.svelte";
import { loadScreensaverState, lockNow } from "../screensaver.svelte";
import { hashPin } from "../screensaver";
import { api } from "../../api/client";
import {
  hideWindowFromCloseConfirm,
  isTauriDesktop,
  openWebInspector,
  reloadWindow,
} from "../../api/desktop";

/// Run a config write and report the outcome as a transient pill, so a
/// launcher command that mutates state still gives feedback without an
/// inline settings panel.
async function withStatus(
  fn: () => Promise<unknown>,
  ok: string,
  fail: string,
): Promise<void> {
  try {
    await fn();
    setTransientStatus(ok);
  } catch {
    setTransientStatus(fail);
  }
}

async function testScreenLock(): Promise<void> {
  await loadScreensaverState();
  lockNow();
}

/// Prompt twice and set the screen-lock PIN. The salt is the workspace
/// root so the same digits hash differently per workspace, matching the
/// About-pane PIN dialog.
async function setScreenLockPin(): Promise<void> {
  const pin = await uiPrompt("Set screen-lock PIN");
  if (pin === null || pin === "") return;
  const again = await uiPrompt("Confirm screen-lock PIN");
  if (again === null) return;
  if (pin !== again) {
    setTransientStatus("PINs did not match");
    return;
  }
  await withStatus(
    async () => {
      const hash = await hashPin(pin, workspace.info?.root ?? "");
      await api.screensaverSetPin(hash);
    },
    "Screen-lock PIN set",
    "Could not set screen-lock PIN",
  );
}

registerCommands([
  {
    id: "app.theme.system",
    title: "Theme: system",
    category: "Global",
    keywords: ["appearance", "auto", "dark", "light"],
    available: () => true,
    run: () => setThemeChoice("system"),
  },
  {
    id: "app.theme.light",
    title: "Theme: light",
    category: "Global",
    keywords: ["appearance"],
    available: () => true,
    run: () => setThemeChoice("light"),
  },
  {
    id: "app.theme.dark",
    title: "Theme: dark",
    category: "Global",
    keywords: ["appearance"],
    available: () => true,
    run: () => setThemeChoice("dark"),
  },
  {
    id: "app.screensaver.enable",
    title: "Screen lock: on",
    category: "Global",
    keywords: ["screensaver", "lock", "privacy"],
    available: () => true,
    run: () =>
      void withStatus(
        () => api.screensaverPatch({ enabled: true }),
        "Screen lock on",
        "Screen lock update failed",
      ),
  },
  {
    id: "app.screensaver.disable",
    title: "Screen lock: off",
    category: "Global",
    keywords: ["screensaver", "lock", "privacy"],
    available: () => true,
    run: () =>
      void withStatus(
        () => api.screensaverPatch({ enabled: false }),
        "Screen lock off",
        "Screen lock update failed",
      ),
  },
  {
    id: "app.screensaver.test",
    title: "Screen lock: test",
    category: "Global",
    keywords: ["screensaver", "lock", "preview"],
    available: () => true,
    run: () => void testScreenLock(),
  },
  {
    id: "app.screensaver.setPin",
    title: "Screen lock: set PIN",
    category: "Global",
    keywords: ["screensaver", "lock", "password", "passcode"],
    available: () => true,
    run: () => void setScreenLockPin(),
  },
  {
    id: "app.screensaver.theme.plain",
    title: "Screen lock theme: default",
    category: "Global",
    keywords: ["screensaver", "plain"],
    available: () => true,
    run: () =>
      void withStatus(
        () => api.screensaverPatch({ theme: "plain" }),
        "Screen lock theme: default",
        "Screen lock update failed",
      ),
  },
  {
    id: "app.screensaver.theme.matrix",
    title: "Screen lock theme: matrix",
    category: "Global",
    keywords: ["screensaver", "matrix", "rain"],
    available: () => true,
    run: () =>
      void withStatus(
        () => api.screensaverPatch({ theme: "matrix" }),
        "Screen lock theme: matrix",
        "Screen lock update failed",
      ),
  },
  {
    // Shares the SHORTCUTS id so the launcher row renders its chord read
    // only. reloadWindow works on web (location.reload) and desktop (IPC).
    id: "app.window.reload",
    title: "Reload",
    category: "Global",
    keywords: ["reload", "refresh", "window"],
    available: () => true,
    run: () => void reloadWindow(),
  },
  {
    // Desktop-only: on web the browser owns DevTools and openWebInspector
    // no-ops, so it is not offered there. Cmd+Opt+I is a Tauri-native chord
    // that bypasses the SPA, so this launcher entry stays chordless.
    id: "app.window.devtools",
    title: "Open Inspector",
    category: "Global",
    keywords: ["devtools", "inspector", "console", "javascript", "debug"],
    available: () => isTauriDesktop(),
    run: () => void openWebInspector(),
  },
  {
    // The close-confirm overlay's Hide answer without the prompt: bury THIS
    // window (sessions stay warm, the record persists hidden and reopens from
    // the launcher). Shares the SHORTCUTS id so the row renders its chord.
    // Desktop-only: the bury IPC is an explicit no-op in a plain browser, so
    // the entry is not offered there.
    id: "app.window.hide",
    title: "Hide window",
    category: "Global",
    keywords: ["hide", "window", "bury", "minimize"],
    available: () => isTauriDesktop(),
    run: () => void hideWindowFromCloseConfirm(),
  },
]);
