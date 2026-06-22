import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";
import graph from "./GraphPanel.svelte?raw";
import fileBrowserSurface from "./FileBrowserSurface.svelte?raw";
import fileTree from "./FileTree.svelte?raw";

describe("file-browser reveal actions", () => {
  test("terminal tab does not leak to the legacy file-browser overlay", () => {
    // The "Show Dir" / "Graph dir" terminal menu entries are gone.
    // Nothing in TerminalTab opens the old overlay.
    expect(terminal).not.toContain("browserOverlay.open = true");
    expect(terminal).not.toContain("function showTerminalCwd()");
    expect(terminal).not.toContain("function graphTerminalCwd()");
  });

  test("graph inspector Open / reveal buttons reveal in a browser TAB", () => {
    // Reveal opens a File Browser TAB via openBrowserInActivePane so
    // the graph tab persists. The overlay-era revealPathInBrowser +
    // close() chain is gone. The dedicated revealSelectedFile /
    // revealSelectedFsEntry helpers were deleted; revealPathInBrowserTab
    // is the single reveal-into-a-new-FB-tab primitive the inspector
    // binds directly on onOpen (file) and onReveal (dir).
    expect(graph).not.toContain("function revealSelectedFile(");
    expect(graph).not.toContain("function revealSelectedFsEntry(");
    expect(graph).toContain("function revealPathInBrowserTab(path: string, isDir: boolean)");
    // File "Open" → revealPathInBrowserTab(path, false): a file selection
    // spawns a File Browser tab with the file selected.
    expect(graph).toContain(
      "() => revealPathInBrowserTab(inspectorSelection.path, false)",
    );
    expect(graph).toContain(
      "onOpen={fsKind === \"file\" ? () => revealPathInBrowserTab(fsPath, false) : undefined}",
    );
    // Directories expand the directory ITSELF (upto = parts.length)
    // so the File Browser opens AT the dir; files expand ancestors.
    expect(graph).toContain(
      "onReveal={fsIsDir ? () => revealPathInBrowserTab(fsPath, true) : undefined}",
    );
    expect(graph).toContain("openBrowserInActivePane(isRoot ? {} : { select: path })");
    // No overlay-era reveal/close leftovers in the reveal path.
    expect(graph).not.toContain("revealPathInBrowser(selectedNode.path");
    expect(graph).not.toContain("revealPathInBrowser(selectedFsNode.path");
    expect(graph).not.toContain("openBrowser().inspectorOpen");
  });
});

// Graph and File Browser are first-class tabs; closing happens via the
// tab strip. Neither surface ships an inline close affordance.
describe("no inline close affordance on first-class surfaces", () => {
  test("GraphPanel chrome has no chrome-btn.close button", () => {
    expect(graph).not.toContain('class="chrome-btn close"');
  });

  // Overlay-era maximize button + scope selector dropdown are gone.
  // Cmd+K + "Graph from here" are the canonical scope-setting paths.
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
    // The old snap-back clobbered a freshly-spawned file:/dir: scope.
    // Synthesizing the ScopeOption from the scopeId prefix replaces it.
    expect(graph).not.toContain("graphState.scopeId = defaultScopeId()");
    expect(graph).toContain("synthesizeScope(graphState.scopeId)");
  });

  // The Graph tab chrome bar is removed entirely. Filter chips and
  // hamburger items live in the tab right-click bubble. GraphPanel is
  // now tab-only.
  test("GraphPanel has no chrome bar (overlay variant removed)", () => {
    expect(graph).not.toContain('<div class="bar">');
    expect(graph).not.toContain("{#if !tab}");
  });

  test("WorkspaceInfoBody renders 'Graph from here' only when onSetAsScope is provided", async () => {
    const workspaceInfo = (
      await import("./WorkspaceInfoBody.svelte?raw")
    ).default as string;
    expect(workspaceInfo).toContain("onSetAsScope");
    // "Graph from here" is a dropdown action in the inspector action model,
    // pushed only when the onSetAsScope prop is present (mirroring the
    // FileInfoBody convention).
    expect(workspaceInfo).toMatch(
      /if \(onSetAsScope\) \{[\s\S]*?secondary\.push\(\{ label: "Graph from here", onClick: onSetAsScope \}\)/,
    );
  });

  // GraphPanel wires onSetAsScope for workspace root and directory nodes.
  // File / tag / mention bodies use the ancestor breadcrumb for re-scope.
  test("GraphPanel wires onReveal + onSetAsScope on WorkspaceInfoBody", () => {
    expect(graph).toMatch(
      /<WorkspaceInfoBody[\s\S]*?onReveal=\{\(\) => revealPathInBrowserTab\("", true\)\}[\s\S]*?onSetAsScope=\{\(\) => graphFromHere\("", true\)\}/,
    );
  });

  test("GraphPanel wires 'Graph from here' for file + directory selections", () => {
    // "Graph from here" is present for both file and folder nodes.
    // The handler is kind-aware: directories re-root to themselves;
    // files re-root to the parent folder.
    expect(graph).toMatch(
      /onSetAsScope=\{[\s\S]*?inspectorSelection\?\.kind === "file" \|\|[\s\S]*?=== "directory"[\s\S]*?graphFromHere\(\s*inspectorSelection\.path,\s*inspectorSelection\.kind === "directory",\s*\)/,
    );
    expect(graph).toMatch(/onSetAsScope=\{\(\) => graphFromHere\(fsPath, fsIsDir\)\}/);
  });

  test("graphFromHere spawns a new tab scoped dir to itself, file to its parent", () => {
    // A directory scopes to dir:<path> (workspace root for "").
    // A file scopes to its parent dir. The old always-parent rule
    // made re-rooting a child folder a no-op and left the inspector blank.
    // The nav contract now spawns a NEW graph tab (openGraphInActivePane)
    // seeded at scopeId + pre-selected on the node, instead of re-rooting
    // the current tab in place.
    expect(graph).toContain("function graphFromHere(path: string, isDir: boolean)");
    expect(graph).toMatch(/if \(isDir\) \{\s*scopeId = path \? `dir:\$\{path\}` : "workspace";/);
    expect(graph).toMatch(/const parent = slash > 0 \? path\.slice\(0, slash\) : ""/);
    expect(graph).toMatch(/scopeId = parent \? `dir:\$\{parent\}` : "workspace"/);
    expect(graph).toMatch(
      /openGraphInActivePane\(\{\s*mode: "semantic",\s*scopeId,\s*depth: 1,\s*pendingSelectId: path,\s*\}\)/,
    );
  });

  test("GraphPanel renders the scope-crumbs ancestor breadcrumb", () => {
    // Breadcrumb gated on scopeAncestors.length > 0 so scopes with
    // no path (tag, global) hide it. Each non-current segment is a
    // button wired to rescopeFromHere.
    expect(graph).toContain("scopeAncestors");
    expect(graph).toMatch(/class="scope-crumbs"/);
    expect(graph).toMatch(/class="crumb"[\s\S]*?onclick=\{\(\) => rescopeFromHere\(crumb\.scopeId\)\}/);
    expect(graph).toMatch(/scopeId: "workspace", current: true/);
  });

  test("GraphPanel rescopeFromHere mutates the current tab (no new spawn)", () => {
    // The breadcrumb click mutates graphState.scopeId in place so the
    // same tab follows the user back up the path. scopeFsGraphFromHere
    // is no longer imported here.
    expect(graph).toContain("function rescopeFromHere(scopeId: string)");
    expect(graph).toContain("graphState.scopeId = scopeId;");
    expect(graph).toContain("graphState.depth = 1;");
    expect(graph).not.toContain("scopeFsGraphFromHere");
  });

  test("FileBrowserSurface spawns a Graph tab from WorkspaceInfoBody", async () => {
    expect(fileBrowserSurface).toContain(
      'onSetAsScope={() => openFsGraphForDirectory("")}',
    );
  });

  test("GraphPanel renders a tab-menu-bubble with mbtn rows + vertical filter rows", () => {
    // The bubble uses standard hamburger-menu row shape (.mbtn) with
    // vertical filter rows (Depth + Reload + per-filter + footer rows).
    // Reload is back as of round 2 (keep-alive manual refetch).
    expect(graph).toMatch(/\{#if tab && tabMenuOpen\}[\s\S]*?class="tab-menu-bubble"/);
    expect(graph).toMatch(
      /class="tab-menu-bubble"[\s\S]*?class="mbtn depth-row"/,
    );
    expect(graph).toMatch(
      /class="tab-menu-bubble"[\s\S]*?onclick=\{reloadGraph\}/,
    );
    expect(graph).toMatch(
      /class="tab-menu-bubble"[\s\S]*?class="mbtn filter-row"[\s\S]*?show\[kind\] = !show\[kind\]/,
    );
    // The horizontal flex .filters chip container belongs to the overlay
    // bar only; the bubble must not carry it.
    expect(graph).not.toMatch(
      /class="tab-menu-bubble"[\s\S]*?<div class="bubble-filters">/,
    );
    expect(graph).not.toMatch(
      /class="tab-menu-bubble"[\s\S]*?onclick=\{toggleInspector\}/,
    );
    expect(graph).not.toMatch(
      /class="tab-menu-bubble"[\s\S]*?onclick=\{doOpenSettings\}/,
    );
    expect(graph).toMatch(
      /class="tab-menu-bubble"[\s\S]*?onclick=\{flipToSettings\}[\s\S]*?<span class="mbtn-label">Settings<\/span>/,
    );
    expect(graph).toMatch(
      /class="tab-menu-bubble"[\s\S]*?onclick=\{doReopenClosedTab\}[\s\S]*?<span class="mbtn-label">Reopen Closed Tab<\/span>/,
    );
    expect(graph).toMatch(
      /class="tab-menu-bubble"[\s\S]*?onclick=\{closeFromMenu\}[\s\S]*?<span class="mbtn-label">Close<\/span>/,
    );
  });

  test("FileBrowserSurface chrome has no chrome-btn.close button", () => {
    expect(fileBrowserSurface).not.toContain('class="chrome-btn close"');
  });
});

// FileBrowserSurface drops the path-display header span (it duplicated
// the tab-strip context). The chrome row collapses to a slim strip.
describe("no path-display header on FileBrowserSurface", () => {
  test('no <span class="name"> in the header', () => {
    expect(fileBrowserSurface).not.toContain('class="name"');
  });

  test("no fileBrowserTitlePath import or browserTitle derived", () => {
    expect(fileBrowserSurface).not.toContain("fileBrowserTitlePath");
    expect(fileBrowserSurface).not.toContain("browserTitle");
  });
});

// Right-docked file browser mirrors row layout so the tree anchors
// against the viewport edge it sits on.
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

// Collapsed-directory chevron mirrors the dock side. Left-dock +
// overlay + tab keep ChevronRight; right-dock flips to ChevronLeft
// (children "open inward"). Expanded chevron stays ChevronDown.
describe("right-docked file browser chevron direction", () => {
  test("FileTree imports ChevronLeft alongside ChevronDown / ChevronRight", () => {
    expect(fileTree).toContain("ChevronLeft");
    expect(fileTree).toContain("ChevronDown");
    expect(fileTree).toContain("ChevronRight");
  });

  test("collapsed-dir chevron branches on rightDock to ChevronLeft vs ChevronRight", () => {
    // Both ChevronLeft (right-dock) and ChevronRight (default) appear
    // for the collapsed state, gated by rightDock. Expanded is always
    // ChevronDown.
    expect(fileTree).toMatch(
      /\{#if expanded\[node\.path\]\}[\s\S]*?<ChevronDown[\s\S]*?\{:else if rightDock\}[\s\S]*?<ChevronLeft[\s\S]*?\{:else\}[\s\S]*?<ChevronRight/,
    );
  });
});
