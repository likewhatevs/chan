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

  // fullstack-64: drop the overlay-era maximize button + the scope
  // selector dropdown. Cmd+K 3 + "Graph from here" are the canonical
  // scope-setting paths now. The snap-back $effect that reset
  // scopeId to defaultScopeId() also goes (was the fullstack-57
  // bug surface).
  test("GraphPanel chrome has no Maximize2 / Minimize2 button", () => {
    expect(graph).not.toContain("<Maximize2");
    expect(graph).not.toContain("<Minimize2");
    expect(graph).not.toContain("doToggleOverlayMaximized");
  });

  test("GraphPanel has no scope-selector dropdown", () => {
    expect(graph).not.toContain('class="scope-select"');
    expect(graph).not.toContain('class="scope-label"');
  });

  test("GraphPanel no longer snaps scopeId back to defaultScopeId on mount", () => {
    // The fullstack-57 bug surface: `if (!currentScope)
    // graphState.scopeId = defaultScopeId()` clobbered a freshly-
    // spawned file:/dir: scope. Synthesizing the ScopeOption from
    // the scopeId prefix replaces the snap-back.
    expect(graph).not.toContain("graphState.scopeId = defaultScopeId()");
    expect(graph).toContain("synthesizeScope(graphState.scopeId)");
  });

  // fullstack-68: kill the Graph tab's chrome bar; filter chips +
  // hamburger items relocate to the tab right-click bubble.
  test("GraphPanel hides the chrome bar when rendered as a tab", () => {
    // The `<div class="bar">` block is now gated on `!tab` so the
    // overlay variant keeps it; the tab variant body is canvas-only.
    expect(graph).toMatch(/\{#if !tab\}[\s\S]*?<div class="bar">/);
  });

  // fullstack-73: "Graph from here" affordance on DriveInfoBody so
  // every inspector surface offers the same action when the drive
  // root is selected.
  test("DriveInfoBody renders 'Graph from here' only when onSetAsScope is provided", async () => {
    const driveInfo = (
      await import("./DriveInfoBody.svelte?raw")
    ).default as string;
    expect(driveInfo).toContain("onSetAsScope");
    expect(driveInfo).toContain(
      'onclick={onSetAsScope}>Graph from here',
    );
    // Button is gated on the prop being present, mirroring the
    // FileInfoBody convention.
    expect(driveInfo).toMatch(/\{#if onSetAsScope\}[\s\S]*?Graph from here/);
  });

  test("GraphPanel passes a re-scope callback to DriveInfoBody", async () => {
    expect(graph).toMatch(
      /<DriveInfoBody\s+onSetAsScope=\{[\s\S]*?scopeFsGraphFromHere\("", true\);[\s\S]*?graphState\.scopeId = "drive";/,
    );
  });

  test("FileBrowserSurface spawns a Graph tab from DriveInfoBody", async () => {
    expect(fileBrowserSurface).toContain(
      'onSetAsScope={() => openFsGraphForDirectory("")}',
    );
  });

  test("GraphPanel renders a tab-menu-bubble carrying menuItems + filterChips", () => {
    // Right-click on the Graph tab opens the bubble; bubble re-uses
    // the existing `menuItems` snippet and a new `filterChips`
    // snippet so chip toggles in the chrome bar AND in the bubble
    // mutate the same `graphState.filters`.
    expect(graph).toContain("{#snippet filterChips()}");
    expect(graph).toMatch(/\{#if tab && tabMenuOpen\}[\s\S]*?class="tab-menu-bubble"/);
    expect(graph).toMatch(
      /class="tab-menu-bubble"[\s\S]*?\{@render menuItems\(\)\}[\s\S]*?\{@render filterChips\(\)\}/,
    );
  });

  test("FileBrowserSurface chrome has no chrome-btn.close button", () => {
    expect(fileBrowserSurface).not.toContain('class="chrome-btn close"');
  });
});

// fullstack-54: FileBrowserSurface drops the path-display header span
// (the `/private/tmp/...` row that duplicated the tab-strip context).
// The chrome row collapses to a slim strip with the kebab on the
// right; no path text in any variant.
describe("fullstack-54: no path-display header on FileBrowserSurface", () => {
  test('no <span class="name"> in the header', () => {
    expect(fileBrowserSurface).not.toContain('class="name"');
  });

  test("no fileBrowserTitlePath import or browserTitle derived", () => {
    expect(fileBrowserSurface).not.toContain("fileBrowserTitlePath");
    expect(fileBrowserSurface).not.toContain("browserTitle");
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
