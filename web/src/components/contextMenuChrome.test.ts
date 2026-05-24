// @vitest-environment jsdom

import { describe, expect, test } from "vitest";
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

  test("pane hover and focus use the same in-out motion channel", () => {
    expect(pane).toMatch(
      /\.pane::before \{[\s\S]*?transform 260ms cubic-bezier\(0\.34, 1\.56, 0\.64, 1\)/,
    );
    expect(pane).toMatch(
      /\.pane:hover::before,\s*\.pane\.focused::before \{[\s\S]*?transform: scale\(1\.006\)/,
    );
    expect(pane).not.toMatch(/\.pane:hover,\s*\.pane\.focused \{[\s\S]*?transform: scale/);
  });
});
