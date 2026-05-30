import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";
import fileBrowserSurface from "./FileBrowserSurface.svelte?raw";
import fileTree from "./FileTree.svelte?raw";
import graph from "./GraphPanel.svelte?raw";

// Right-click menu trims across Terminal / FB / Graph + FB
// click-to-inspector for tab + overlay variants. Search (Cmd+K f)
// and Settings (Cmd+,) are global keystrokes and are not duplicated
// in per-tab menus. Show/Hide Details is redundant once row-clicks
// auto-open the inspector in tab + overlay variants.

describe("TerminalTab right-click: Search still gone; Settings comes back as flip", () => {
  test("no Search menu entry (Cmd+K f is the global surface)", () => {
    expect(terminal).not.toContain('onclick={openSearch}');
    expect(terminal).not.toMatch(/<span class="mbtn-label">Search<\/span>/);
  });

  test("no openSettingsFromMenu entry (the global Settings overlay opener)", () => {
    // `-80` dropped duplicating the global Settings chord
    // (Cmd+,) in per-tab menus. That rule stands - no
    // `openSettingsFromMenu` style handler in the source.
    expect(terminal).not.toContain("onclick={openSettingsFromMenu}");
  });

  test("Settings (flip) entry is present and routes to flipToSettings", () => {
    // A Settings entry flips the tab to its back-side config view
    // (HybridTerminalConfig). This is a per-tab flip, not a global
    // shortcut duplicate; the no-global-duplicate rule is preserved.
    expect(terminal).toContain("onclick={flipToSettings}");
    expect(terminal).toMatch(/<span class="mbtn-label">Settings<\/span>/);
  });

  test("Find row stays (unrelated; lives behind a different chord)", () => {
    expect(terminal).toContain('onclick={openFind}');
    expect(terminal).toMatch(/<span class="mbtn-label">Find<\/span>/);
  });
});

describe("FileBrowserSurface menu drops Search this + Settings + Show/Hide Details", () => {
  test("no Search this entry", () => {
    expect(fileBrowserSurface).not.toContain('onclick={searchWorkspace}');
    expect(fileBrowserSurface).not.toContain(">Search this<");
  });

  test("no Settings entry", () => {
    expect(fileBrowserSurface).not.toContain('onclick={doOpenSettings}');
  });

  test("no Show/Hide Details entry (auto-opens on click in tab+overlay)", () => {
    expect(fileBrowserSurface).not.toContain('onclick={toggleInspector}');
    expect(fileBrowserSurface).not.toContain('"Hide Details"');
    expect(fileBrowserSurface).not.toContain('"Show Details"');
  });
});

describe("FB row click auto-opens inspector for tab + overlay only", () => {
  test("FileTree.selectPath no longer pokes browserOverlay.inspectorOpen directly", () => {
    // The auto-open call moved to FileBrowserSurface so it can gate
    // on variant. FileTree just emits the click via `onClickRow`.
    expect(fileTree).not.toContain("browserOverlay.inspectorOpen = true");
    expect(fileTree).toContain("onClickRow?.(path)");
  });

  test("FileTree exposes an onClickRow prop", () => {
    expect(fileTree).toContain("onClickRow?: (path: string) => void");
  });

  test("FileBrowserSurface onRowClicked opens the inspector for tab + overlay variants", () => {
    expect(fileBrowserSurface).toContain("function onRowClicked");
    expect(fileBrowserSurface).toContain("if (isTab || isOverlay) browserState.inspectorOpen = true");
    expect(fileBrowserSurface).toContain("onClickRow={onRowClicked}");
  });
});

describe("FB dock menu drops the Open overlay entry", () => {
  test("no `Open overlay` label survives in any variant", () => {
    expect(fileBrowserSurface).not.toContain(">Open overlay<");
  });

  test("the dock-variant gate for Open overlay is gone", () => {
    // -82 dropped the `{#if variant === "dock"}` block that
    // wrapped the entry. After the drop, no `#if variant === "dock"`
    // gate exists in the menuItems snippet.
    expect(fileBrowserSurface).not.toContain('onclick={openOverlay}');
  });

  test("openOverlay handler dropped (only consumer was the removed entry)", () => {
    expect(fileBrowserSurface).not.toContain("function openOverlay()");
    expect(fileBrowserSurface).toContain("function openCurrentInFileBrowser()");
  });
});

describe("GraphPanel drops inspector/global Settings and keeps flip footer", () => {
  test("bubble does not invoke toggleInspector", () => {
    expect(graph).not.toMatch(
      /class="tab-menu-bubble"[\s\S]*?onclick=\{toggleInspector\}/,
    );
  });

  test("bubble does not invoke doOpenSettings", () => {
    expect(graph).not.toMatch(
      /class="tab-menu-bubble"[\s\S]*?onclick=\{doOpenSettings\}/,
    );
  });

  test("menuItems snippet also drops Show Details + global Settings rows", () => {
    expect(graph).not.toContain('onclick={toggleInspector}');
    expect(graph).not.toContain('onclick={doOpenSettings}');
  });

  test("Depth slider + Reload + -a footer stay", () => {
    expect(graph).toContain('class="mbtn depth-row"');
    expect(graph).toContain('onclick={reloadGraph}');
    expect(graph).toContain('onclick={flipToSettings}');
    expect(graph).toContain('onclick={doReopenClosedTab}');
    expect(graph).toContain('onclick={closeFromMenu}');
  });
});
