import { describe, expect, test } from "vitest";
import fileEditorSource from "./FileEditorTab.svelte?raw";
import terminalSource from "./TerminalTab.svelte?raw";

// `fullstack-b-26`: every per-tab right-click menu surfaces a
// Reload + Open Inspector entry at the tail, paired with the
// `reload_window` + `open_devtools` IPCs that `-b-17` + `-a-36`
// plumbed for the pane-context menu. These pins guard the wiring
// in the source so a future menu refactor can't silently drop the
// entries without flagging.

// `fullstack-a-67f`: addendum-a's verbatim Editor menu spec
// also drops the `-b-26` Reload + Open Inspector tail entries,
// matching `-a-67d`'s Terminal drop. These pins flip from
// REQUIRE to FORBID so a regression that re-adds them gets
// caught. Cmd+R + the pane hamburger still surface window-level
// reload + devtools.
describe("fullstack-a-67f: file-editor tab right-click — Reload + Open Inspector dropped", () => {
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
