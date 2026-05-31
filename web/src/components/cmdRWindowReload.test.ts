import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import pane from "./Pane.svelte?raw";
import shortcuts from "../state/shortcuts.ts?raw";

// Window-level reload via the `reloadWindow()` helper. macOS binds
// Cmd+R; Linux/Windows binds Ctrl+Shift+R so plain Ctrl+R stays with the
// shell's reverse-search. The pane right-click menu's Reload entry shows
// the OS-resolved chord label.

describe("reload chord registry entry", () => {
  test("app.window.reload chord descriptor present in shortcuts registry", () => {
    expect(shortcuts).toMatch(
      /id: "app\.window\.reload",[\s\S]*?label: "Reload window",[\s\S]*?web: "Mod\+R",[\s\S]*?native: "Mod\+R",/,
    );
  });

  test("descriptor documents the Linux/Windows Ctrl+Shift+R divergence", () => {
    expect(shortcuts).toMatch(
      /id: "app\.window\.reload",[\s\S]*?note: "Ctrl\+Shift\+R on Linux \/ Windows",/,
    );
  });

  test("osChord moves reload off plain Ctrl+R on non-macOS", () => {
    // Mod+Shift+R -> Ctrl+Shift+R once Mod renders as Ctrl; plain Ctrl+R
    // is never the reload chord off macOS.
    expect(shortcuts).toMatch(
      /RELOAD_SHORTCUT_ID && os !== "mac"\) return "Mod\+Shift\+R";/,
    );
  });
});

describe("App.svelte keymap binding", () => {
  test("reloadWindow imported from api/desktop", () => {
    // The desktop import also carries isTauriDesktop +
    // requestCloseWindow, so match reloadWindow within the
    // named-import list rather than the exact single-name form.
    expect(app).toMatch(
      /import \{[^}]*\breloadWindow\b[^}]*\} from "\.\/api\/desktop";/,
    );
  });

  test("reload handler branches per-OS and dispatches reloadWindow()", () => {
    // macOS: Cmd+R. Non-macOS: Ctrl+Shift+R (so plain Ctrl+R is left for
    // the terminal). preventDefault + void reloadWindow() on match.
    expect(app).toMatch(
      /currentOS\(\) === "mac"[\s\S]*?e\.metaKey && !e\.ctrlKey && !e\.altKey && !e\.shiftKey && e\.code === "KeyR"[\s\S]*?e\.ctrlKey && e\.shiftKey && !e\.metaKey && !e\.altKey && e\.code === "KeyR"/,
    );
    expect(app).toMatch(
      /if \(reloadChord\) \{[\s\S]*?e\.preventDefault\(\);[\s\S]*?void reloadWindow\(\);/,
    );
  });
});

describe("Pane.svelte menu annotation", () => {
  test("Reload menu entry renders the chord label via the registry", () => {
    expect(pane).toMatch(
      /onclick=\{doReloadPane\}[\s\S]*?<span class="menu-row-label">Reload<\/span>[\s\S]*?<span class="menu-row-chord">\{chordLabel\("app\.window\.reload"\)\}<\/span>/,
    );
  });

  test("Reload menu entry routes through reloadWindow()", () => {
    expect(pane).toMatch(/async function doReloadPane\(\)/);
    expect(pane).toMatch(/await reloadWindow\(\)/);
  });
});
