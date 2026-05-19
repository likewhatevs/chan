import { describe, expect, test } from "vitest";
import fileBrowserSurface from "./FileBrowserSurface.svelte?raw";

// fullstack-67: the FB surface drops its <header> entirely in tab
// variant (per @@Alex's "no stacked hamburgers" call). FB-specific
// menu items still render via a hidden HamburgerMenu that the
// tab-strip right-click handler in Pane.svelte opens at the cursor
// through the shared `tabMenu` state. Dock + Overlay variants keep
// the slim chrome row from `-54`.

describe("fullstack-67: FileBrowserSurface header gated on tab variant", () => {
  test("header is wrapped in {#if !isTab}", () => {
    expect(fileBrowserSurface).toMatch(/{\s*#if\s+!isTab\s*}[\s\S]*?<header>/);
  });

  test("tab variant renders a triggerless HamburgerMenu instead of the header", () => {
    // The {:else} branch (when isTab) mounts a HamburgerMenu with
    // showTrigger={false} so the FB-specific items are reachable
    // via openAtCursor without painting a visible trigger.
    expect(fileBrowserSurface).toMatch(
      /{\s*:else\s*}[\s\S]*?<HamburgerMenu[\s\S]*?showTrigger=\{false\}/,
    );
  });

  test("an $effect mirrors tab-strip right-click into menu.openAtCursor", () => {
    // The tab-strip right-click in Pane.svelte calls
    // `openTabMenu(t.id, anchor)` which sets `tabMenu.openForTabId`
    // and `tabMenu.anchor`. The surface's effect mirrors that back
    // into `menu.openAtCursor()` so FB-specific items still render
    // at the cursor on tab-strip right-click in tab variant.
    expect(fileBrowserSurface).toContain("tabMenu.openForTabId");
    expect(fileBrowserSurface).toContain("tabMenu.anchor");
    expect(fileBrowserSurface).toContain("menu?.openAtCursor(anchor.left, anchor.top)");
  });
});
