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

describe("fullstack-b-26: terminal-tab right-click — Reload + Open Inspector", () => {
  test("menu source ships a Reload entry wired to reloadWindow", () => {
    expect(terminalSource).toMatch(/<span class="mbtn-label">Reload<\/span>/);
    expect(terminalSource).toMatch(/onclick=\{doReloadWindow\}/);
    expect(terminalSource).toMatch(/await reloadWindow\(\)/);
  });

  test("menu source ships an Open Inspector entry wired to openWebInspector", () => {
    expect(terminalSource).toMatch(
      /<span class="mbtn-label">Open Inspector<\/span>/,
    );
    expect(terminalSource).toMatch(/onclick=\{doOpenInspector\}/);
    expect(terminalSource).toMatch(/await openWebInspector\(\)/);
  });

  test("desktop helpers imported from the api/desktop seam", () => {
    expect(terminalSource).toMatch(/from "\.\.\/api\/desktop"/);
    expect(terminalSource).toMatch(/reloadWindow,/);
    expect(terminalSource).toMatch(/openWebInspector,/);
    expect(terminalSource).toMatch(/isTauriDesktop,/);
  });

  test("web-mode inspector fallback toasts a notify() hint", () => {
    expect(terminalSource).toMatch(/notify\(/);
    expect(terminalSource).toMatch(/Use the browser's built-in inspector/);
  });
});
