import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import pane from "./Pane.svelte?raw";
import shortcuts from "../state/shortcuts.ts?raw";

// `fullstack-a-73`: Cmd+R global chord → window-level reload via
// existing `reloadWindow()` helper. Annotates the pane right-click
// menu's Reload entry with the chord label.

describe("fullstack-a-73: chord registry entry", () => {
  test("app.window.reload chord descriptor present in shortcuts registry", () => {
    expect(shortcuts).toMatch(
      /id: "app\.window\.reload",[\s\S]*?label: "Reload window",[\s\S]*?web: "Mod\+R",[\s\S]*?native: "Mod\+R",/,
    );
  });
});

describe("fullstack-a-73: App.svelte keymap binding", () => {
  test("reloadWindow imported from api/desktop", () => {
    expect(app).toMatch(/import \{ reloadWindow \} from "\.\/api\/desktop";/);
  });

  test("Cmd+R handler dispatches reloadWindow() and preventDefault", () => {
    expect(app).toMatch(
      /if \(meta && !e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyR"\) \{[\s\S]*?e\.preventDefault\(\);[\s\S]*?void reloadWindow\(\);/,
    );
  });
});

describe("fullstack-a-73: Pane.svelte menu annotation", () => {
  test("Reload menu entry renders the chord label via the registry", () => {
    expect(pane).toMatch(
      /onclick=\{doReloadPane\}[\s\S]*?<span class="menu-row-label">Reload<\/span>[\s\S]*?<span class="menu-row-chord">\{chordLabel\("app\.window\.reload"\)\}<\/span>/,
    );
  });

  test("comment block documents the dual entry point + chan-desktop defense-in-depth", () => {
    expect(pane).toMatch(
      /`fullstack-a-73`: window-level reload, like a browser[\s\S]*?serve\.rs:1140 Tauri-side[\s\S]*?defense-in-depth fallback/,
    );
  });
});
