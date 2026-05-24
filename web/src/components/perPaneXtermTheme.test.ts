import { describe, expect, test } from "vitest";
import terminalTab from "./TerminalTab.svelte?raw";
import graphCanvas from "./GraphCanvas.svelte?raw";

// fullstack-78 plus Track C: surface body theme overrides need to
// propagate to JS-themed surfaces that don't follow the CSS cascade.
// xterm.js renders to its own canvas with a theme object set at
// construction; GraphCanvas re-reads CSS tokens on a MutationObserver
// tick. Both need to see surface-level data-theme changes, not just
// the document root.

describe("Track C: TerminalTab tracks terminal surface body theme", () => {
  test("$effect reads the effective terminal surface theme", () => {
    expect(terminalTab).toContain('effectiveHybridSurfaceTheme("terminal")');
    expect(terminalTab).toContain(
      'data-theme={surfaceThemeOverride("terminal")}',
    );
  });

  test("effective theme is resolved through the shared store", () => {
    expect(terminalTab).toContain("function effectiveTerminalTheme()");
    expect(terminalTab).toContain('return effectiveHybridSurfaceTheme("terminal")');
  });

  test("terminalTheme() branches on effective terminal theme", () => {
    expect(terminalTab).toContain("const effective = effectiveTerminalTheme()");
    expect(terminalTab).toContain('if (effective === "light")');
  });

  test("terminalTheme() reads CSS variables from host, not document root", () => {
    expect(terminalTab).toContain("getComputedStyle(host ?? document.documentElement)");
  });
});

describe("Track C: GraphCanvas MutationObserver watches graph body theme", () => {
  test("observer attaches to the nearest graph-tab in addition to documentElement", () => {
    expect(graphCanvas).toContain('containerEl.closest(".graph-tab")');
    expect(graphCanvas).toContain('attributeFilter: ["data-theme"]');
  });
});
