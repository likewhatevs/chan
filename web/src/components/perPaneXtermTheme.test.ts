import { describe, expect, test } from "vitest";
import terminalTab from "./TerminalTab.svelte?raw";
import graphCanvas from "./GraphCanvas.svelte?raw";

// fullstack-78: per-pane theme overrides from -59 need to propagate
// to JS-themed surfaces that don't follow the CSS cascade. xterm.js
// renders to its own canvas with a theme object set at construction;
// the GraphCanvas D3 layer re-reads CSS tokens on a MutationObserver
// tick. Both need to see pane-level data-theme changes, not just the
// document root.

describe("fullstack-78: TerminalTab tracks pane-local theme override", () => {
  test("$effect reads layout.nodes[paneId]?.theme alongside ui.theme", () => {
    // The original -59 effect tracked only `ui.theme`; the fix adds
    // the pane node's theme so per-pane flips re-apply xterm's
    // theme without a remount.
    expect(terminalTab).toContain("ui.theme");
    expect(terminalTab).toContain("layout.nodes[paneId]");
    expect(terminalTab).toMatch(/void\s+node\.theme/);
  });

  test("effective theme falls back to ui.theme when pane has no override", () => {
    expect(terminalTab).toContain("function effectivePaneTheme()");
    expect(terminalTab).toContain('if (node?.kind === "leaf" && node.theme) return node.theme');
    expect(terminalTab).toContain("return ui.theme");
  });

  test("terminalTheme() branches on effective pane theme, not global", () => {
    // The light/dark palette branch must compare against the
    // effective theme so a pane override picks the right named
    // colours.
    expect(terminalTab).toContain("const effective = effectivePaneTheme()");
    expect(terminalTab).toContain('if (effective === "light")');
  });

  test("terminalTheme() reads CSS variables from host, not document root", () => {
    // `host` is the terminal's container element, inside the pane —
    // its computed style picks up the pane's data-theme cascade.
    // The original implementation read from
    // `document.documentElement`, which misses the per-pane
    // override.
    expect(terminalTab).toContain("getComputedStyle(host ?? document.documentElement)");
  });
});

describe("fullstack-78: GraphCanvas MutationObserver watches the pane's data-theme", () => {
  test("observer attaches to the nearest .pane ancestor in addition to documentElement", () => {
    expect(graphCanvas).toContain('containerEl.closest(".pane")');
    expect(graphCanvas).toContain('attributeFilter: ["data-theme"]');
  });
});
