import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import pane from "./Pane.svelte?raw";
import shortcuts from "../state/shortcuts.ts?raw";

// Cmd+R global chord → window-level reload via the `reloadWindow()`
// helper. The pane right-click menu's Reload entry shows the chord
// label.

describe("Cmd+R chord registry entry", () => {
  test("app.window.reload chord descriptor present in shortcuts registry", () => {
    expect(shortcuts).toMatch(
      /id: "app\.window\.reload",[\s\S]*?label: "Reload window",[\s\S]*?web: "Mod\+R",[\s\S]*?native: "Mod\+R",/,
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

  test("Cmd+R handler dispatches reloadWindow() and preventDefault", () => {
    expect(app).toMatch(
      /if \(meta && !e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyR"\) \{[\s\S]*?e\.preventDefault\(\);[\s\S]*?void reloadWindow\(\);/,
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
