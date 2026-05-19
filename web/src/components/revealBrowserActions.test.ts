import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";
import graph from "./GraphPanel.svelte?raw";
import fileBrowserSurface from "./FileBrowserSurface.svelte?raw";
import fileTree from "./FileTree.svelte?raw";

describe("file-browser reveal actions", () => {
  test("terminal tab does not leak to the legacy file-browser overlay", () => {
    // `fullstack-42` dropped the "Show Dir" / "Graph dir" entries
    // from the terminal hamburger menu (Pane Mode + context covers
    // them). Their click handlers were removed too; what stays is
    // the rule that NOTHING in TerminalTab opens the old overlay.
    expect(terminal).not.toContain("browserOverlay.open = true");
    expect(terminal).not.toContain("function showTerminalCwd()");
    expect(terminal).not.toContain("function graphTerminalCwd()");
  });

  test("graph inspector reveal buttons reveal in a browser tab", () => {
    expect(graph).toContain("function revealSelectedFile()");
    expect(graph).toContain("revealPathInBrowser(selectedNode.path, { inspectorOpen: true });");
    expect(graph).toContain("function revealSelectedFsEntry()");
    expect(graph).toContain("revealPathInBrowser(selectedFsNode.path, { inspectorOpen: true });");
    expect(graph).not.toContain("openBrowser().inspectorOpen");
  });
});

// The Graph and File Browser surfaces are now first-class tabs; closing
// happens via the tab strip's `×`, so neither surface should ship an
// inline close affordance in its own chrome.
describe("no inline close affordance on first-class surfaces", () => {
  test("GraphPanel chrome has no chrome-btn.close button", () => {
    expect(graph).not.toContain('class="chrome-btn close"');
  });

  test("FileBrowserSurface chrome has no chrome-btn.close button", () => {
    expect(fileBrowserSurface).not.toContain('class="chrome-btn close"');
  });
});

// fullstack-38: right-docked file browser mirrors row layout so the
// tree visually anchors against whichever viewport edge it sits on.
describe("right-docked file browser mirrors text alignment", () => {
  test("FileBrowserSurface forwards dockSide=right to FileTree only in dock variant", () => {
    expect(fileBrowserSurface).toContain(
      'dockSide={variant === "dock" ? side : undefined}',
    );
  });

  test("FileTree accepts a dockSide prop and toggles the right-dock class", () => {
    expect(fileTree).toContain('dockSide?: "left" | "right"');
    expect(fileTree).toContain("class:right-dock={rightDock}");
  });

  test("FileTree swaps inline padding from left to right under right-dock", () => {
    // The dir / file / empty rows must conditionally render
    // padding-right (right-dock) vs padding-left (default) so the
    // indent column lands on the side opposite the chevron.
    expect(fileTree).toContain("rightDock");
    expect(fileTree).toContain("padding-right: ${depth * 12}px");
    expect(fileTree).toContain("padding-right: ${depth * 12 + 16}px");
  });

  test("FileTree CSS reverses row order and right-aligns the name under right-dock", () => {
    expect(fileTree).toContain(".tree.right-dock .row");
    expect(fileTree).toContain("flex-direction: row-reverse");
    expect(fileTree).toContain(".tree.right-dock .name");
    expect(fileTree).toContain("text-align: right");
  });
});

// fullstack-49: collapsed-directory chevron mirrors with the dock side.
// Left-dock + overlay + tab variants keep ChevronRight; right-dock
// flips to ChevronLeft because the mirrored row layout reads children
// as "opening inward" toward the left. Expanded chevron stays
// ChevronDown — symmetric on the horizontal axis.
describe("right-docked file browser chevron direction", () => {
  test("FileTree imports ChevronLeft alongside ChevronDown / ChevronRight", () => {
    expect(fileTree).toContain("ChevronLeft");
    expect(fileTree).toContain("ChevronDown");
    expect(fileTree).toContain("ChevronRight");
  });

  test("collapsed-dir chevron branches on rightDock to ChevronLeft vs ChevronRight", () => {
    // The render block must include both ChevronLeft (right-dock
    // variant) and ChevronRight (default) for the collapsed state,
    // gated by the rightDock flag. The expanded state stays
    // ChevronDown unconditionally.
    expect(fileTree).toMatch(
      /\{#if expanded\[node\.path\]\}[\s\S]*?<ChevronDown[\s\S]*?\{:else if rightDock\}[\s\S]*?<ChevronLeft[\s\S]*?\{:else\}[\s\S]*?<ChevronRight/,
    );
  });
});
