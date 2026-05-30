import { describe, expect, test } from "vitest";
import fileEditorSource from "./FileEditorTab.svelte?raw";
import terminalSource from "./TerminalTab.svelte?raw";

// Reload + Open Inspector tail entries were dropped from the file editor
// and terminal tab menus. Cmd+R + the pane hamburger still surface
// window-level reload + devtools. These pins flip from REQUIRE to FORBID.
describe("file-editor tab right-click: Reload + Open Inspector dropped", () => {
  test("no Reload entry in the editor menu", () => {
    expect(fileEditorSource).not.toMatch(
      /<span class="mbtn-label">Reload<\/span>/,
    );
    expect(fileEditorSource).not.toMatch(/onclick=\{doReloadWindow\}/);
  });

  test("no Open Inspector entry in the editor menu", () => {
    expect(fileEditorSource).not.toMatch(
      /<span class="mbtn-label">Open Inspector<\/span>/,
    );
    expect(fileEditorSource).not.toMatch(/onclick=\{doOpenInspector\}/);
  });

  test("desktop helpers no longer imported", () => {
    expect(fileEditorSource).not.toMatch(/from "\.\.\/api\/desktop"/);
  });

  test("inspector-fallback notify() hint gone too", () => {
    expect(fileEditorSource).not.toMatch(
      /Use the browser's built-in inspector/,
    );
  });
});

describe("terminal-tab right-click: Reload + Open Inspector dropped", () => {
  test("no Reload entry in the terminal menu", () => {
    expect(terminalSource).not.toMatch(
      /<span class="mbtn-label">Reload<\/span>/,
    );
    expect(terminalSource).not.toMatch(/onclick=\{doReloadWindow\}/);
  });

  test("no Open Inspector entry in the terminal menu", () => {
    expect(terminalSource).not.toMatch(
      /<span class="mbtn-label">Open Inspector<\/span>/,
    );
    expect(terminalSource).not.toMatch(/onclick=\{doOpenInspector\}/);
  });

  test("desktop helpers no longer imported", () => {
    expect(terminalSource).not.toMatch(/from "\.\.\/api\/desktop"/);
  });

  test("inspector-fallback notify() hint gone too", () => {
    expect(terminalSource).not.toMatch(
      /Use the browser's built-in inspector/,
    );
  });
});
