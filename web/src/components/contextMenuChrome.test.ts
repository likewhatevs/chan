// @vitest-environment jsdom

import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import editor from "./FileEditorTab.svelte?raw";
import fileTree from "./FileTree.svelte?raw";
import graph from "./GraphPanel.svelte?raw";
import hamburger from "./HamburgerMenu.svelte?raw";
import pane from "./Pane.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";
import { portal } from "./portal";

describe("context menu chrome", () => {
  test("custom tab menus portal before clamping to viewport coordinates", () => {
    for (const source of [terminal, editor, graph]) {
      expect(source).toMatch(/use:portal\s+use:clampMenu=/);
      expect(source).toMatch(/z-index: 25500;/);
    }
  });

  test("shared portal action moves a menu node under document.body", () => {
    const node = document.createElement("div");
    const action = portal(node);

    expect(document.body.contains(node)).toBe(true);

    action.destroy();
    expect(document.body.contains(node)).toBe(false);
  });

  test("hamburger and file tree use the shared portal action", () => {
    expect(hamburger).toContain('import { portal } from "./portal";');
    expect(fileTree).toContain('import { portal } from "./portal";');
  });

  test("right-click menu rows share the tab-pill hover motion curve", () => {
    for (const source of [terminal, editor, graph, fileTree, hamburger]) {
      expect(source).toContain("transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1)");
      expect(source).toContain("transform: scale(1.02)");
    }
  });

  test("pane focus wobble shares the right-click menu motion curve", () => {
    // Wobble on a newly focused pane uses the same easeOutBack curve
    // as the tab-pill / right-click menu pops above, so the motion
    // language stays consistent across surfaces. The animation lives
    // on `.pane.focused.wobble` (not on `::before`) so the chrome
    // halo can pulse out via box-shadow.
    expect(pane).toMatch(
      /\.pane\.focused\.wobble \{[\s\S]*?animation: pane-wobble-once 360ms cubic-bezier\(0\.34, 1\.56, 0\.64, 1\)/,
    );
    // xterm's WebGL glyph atlas can corrupt if an ancestor pane is
    // scaled during focus changes. Guard against any future
    // refactor re-introducing transform: scale on the pane element
    // itself (hover, focused, or wobble).
    expect(pane).not.toMatch(/\.pane(:hover|\.focused|\.wobble)?\s*\{[\s\S]*?transform: scale/);
  });

  test("Hybrid Nav focus chrome does not composite pane bodies", () => {
    expect(app).not.toMatch(/\.app\.pane-mode\s+:global\(\.pane:not\(\.focused\)\)\s*\{[\s\S]*?filter:/);
    expect(app).not.toMatch(/\.app\.pane-mode\s+:global\(\.pane:not\(\.focused\)\)\s*\{[\s\S]*?opacity:/);
  });
});
