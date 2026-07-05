import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import { TERMINAL_ONLY_COMMANDS } from "../state/windowMode";

// WP17 App-level wiring: the `app.window.confirmClose` command (evaled by the
// desktop host on an OS red-dot) closes straight away while reconnecting or when
// the window is empty, and otherwise opens the 3-way overlay. Pinned against the
// source, matching paneModeKeymap.test.ts / ctrlDCloseTab.test.ts style.
describe("app.window.confirmClose dispatch", () => {
  test("mounts the overlay and accepts the command in terminal-only windows", () => {
    expect(app).toContain('import CloseConfirmOverlay from "./components/CloseConfirmOverlay.svelte";');
    expect(app).toContain("<CloseConfirmOverlay />");
    // A standalone/control terminal window is terminal-only, so confirmClose
    // must survive the terminal-only command filter, whose allowlist lives in
    // state/windowMode.ts (App.svelte's runCommand consults it).
    expect(TERMINAL_ONLY_COMMANDS.has("app.window.confirmClose")).toBe(true);
  });

  test("closes now while reconnecting or when the window has no tabs", () => {
    const arm = app
      .split('case "app.window.confirmClose":')
      .at(1)
      ?.split('case "app.file.new":')
      .at(0);
    expect(arm).toBeTruthy();
    expect(arm).toContain("if (ui.disconnectBlocking || !hasAnyTab()) {");
    expect(arm).toContain("discardWindowSession({ reap: true });");
    expect(arm).toContain("if (isTauriDesktop()) void requestCloseWindow();");
  });

  test("otherwise opens the 3-way overlay", () => {
    const arm = app
      .split('case "app.window.confirmClose":')
      .at(1)
      ?.split('case "app.file.new":')
      .at(0);
    expect(arm).toContain("void uiCloseConfirm();");
  });
});

// Web parity is unchanged: there is no OS red-dot on the web, closing a browser
// tab is a Hide (the beforeunload/pagehide handlers flush only, the saved blob
// survives), and the explicit "close window" command still clears all tabs and
// DELETEs the blob with requestCloseWindow gated to desktop. WP17 adds no web
// behavior, so pin the existing mapping.
describe("web close/hide parity (unchanged)", () => {
  test("browser tab close flushes buffers (a Hide), it does not discard", () => {
    expect(app).toContain('window.addEventListener("beforeunload", onUnloadFlushBuffers)');
    expect(app).toContain('window.addEventListener("pagehide", onUnloadFlushBuffers)');
  });

  test("explicit close-window still discards + gates the destroy to desktop", () => {
    const arm = app
      .split('case "app.window.close":')
      .at(1)
      ?.split('case "app.window.confirmClose":')
      .at(0);
    expect(arm).toBeTruthy();
    expect(arm).toContain("discardWindowSession();");
    expect(arm).toContain("if (isTauriDesktop()) void requestCloseWindow();");
  });
});
