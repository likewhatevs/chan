import { describe, expect, test } from "vitest";
import fileEditorSource from "./FileEditorTab.svelte?raw";
import terminalSource from "./TerminalTab.svelte?raw";

// `fullstack-b-26`: every per-tab right-click menu surfaces a
// Reload + Open Inspector entry at the tail, paired with the
// `reload_window` + `open_devtools` IPCs that `-b-17` + `-a-36`
// plumbed for the pane-context menu. These pins guard the wiring
// in the source so a future menu refactor can't silently drop the
// entries without flagging.

describe("fullstack-b-26: file-editor tab right-click — Reload + Open Inspector", () => {
  test("menu source ships a Reload entry wired to reloadWindow", () => {
    expect(fileEditorSource).toMatch(/<span class="mbtn-label">Reload<\/span>/);
    expect(fileEditorSource).toMatch(/onclick=\{doReloadWindow\}/);
    expect(fileEditorSource).toMatch(/await reloadWindow\(\)/);
  });

  test("menu source ships an Open Inspector entry wired to openWebInspector", () => {
    expect(fileEditorSource).toMatch(
      /<span class="mbtn-label">Open Inspector<\/span>/,
    );
    expect(fileEditorSource).toMatch(/onclick=\{doOpenInspector\}/);
    expect(fileEditorSource).toMatch(/await openWebInspector\(\)/);
  });

  test("desktop helpers imported from the api/desktop seam", () => {
    expect(fileEditorSource).toMatch(/from "\.\.\/api\/desktop"/);
    expect(fileEditorSource).toMatch(/reloadWindow,/);
    expect(fileEditorSource).toMatch(/openWebInspector,/);
    expect(fileEditorSource).toMatch(/isTauriDesktop,/);
  });

  test("web-mode inspector fallback toasts a notify() hint", () => {
    // The user sees a meaningful hint pointing at the browser's
    // built-in inspector when openWebInspector() returns false on
    // the web build. Same shape as Pane.svelte's doOpenInspector.
    expect(fileEditorSource).toMatch(/notify\(/);
    expect(fileEditorSource).toMatch(/Use the browser's built-in inspector/);
  });
});

// `fullstack-a-67d`: addendum-a's verbatim Terminal menu spec
// drops the `-b-26` Reload + Open Inspector tail entries. These
// pins flip from REQUIRE to FORBID so a regression that
// re-adds them gets caught. Cmd+R and the pane hamburger
// still surface window-level reload + devtools.
describe("fullstack-a-67d: terminal-tab right-click — Reload + Open Inspector dropped", () => {
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
