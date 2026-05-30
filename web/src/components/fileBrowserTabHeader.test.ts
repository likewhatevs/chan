import { describe, expect, test } from "vitest";
import fileBrowserSurface from "./FileBrowserSurface.svelte?raw";

// fullstack-67: tab variant drops its <header>. FB-specific menu
// items render via a hidden HamburgerMenu opened at the cursor by
// the tab-strip right-click handler in Pane.svelte through the
// shared `tabMenu` state.
//
// fullstack-71: dock variant also drops its <header>. The
// triggerless HamburgerMenu is shared with the tab variant;
// dock-body right-click hits `.browser` root's existing
// `oncontextmenu={onBrowserContextMenu}` which calls
// `menu.openAtCursor` directly. Only Overlay variant keeps a
// surface header (close + maximize + kebab — the maximize
// affordance has nowhere else to live).

describe("FileBrowserSurface header is overlay-only", () => {
  test("header is wrapped in {#if isOverlay}", () => {
    expect(fileBrowserSurface).toMatch(/{\s*#if\s+isOverlay\s*}[\s\S]*?<header>/);
  });

  test("non-overlay variants (tab + dock) render a triggerless HamburgerMenu instead of the header", () => {
    // The {:else} branch (when !isOverlay, i.e. tab OR dock) mounts
    // a HamburgerMenu with showTrigger={false} so the FB-specific
    // items are reachable via openAtCursor without painting a
    // visible trigger.
    expect(fileBrowserSurface).toMatch(
      /{\s*:else\s*}[\s\S]*?<HamburgerMenu[\s\S]*?showTrigger=\{false\}/,
    );
  });

  test("an $effect mirrors tab-strip right-click into menu.openAtCursor", () => {
    // Tab-strip right-click in Pane.svelte calls
    // `openTabMenu(t.id, anchor)`; the effect mirrors that back into
    // `menu.openAtCursor()` so FB-specific items still render on the
    // tab variant.
    expect(fileBrowserSurface).toContain("tabMenu.openForTabId");
    expect(fileBrowserSurface).toContain("tabMenu.anchor");
    expect(fileBrowserSurface).toContain("menu?.openAtCursor(anchor.left, anchor.top)");
  });

  test("dock-body right-click flows through onBrowserContextMenu → menu.openAtCursor", () => {
    // Dock variant has no tab strip; the dock-body `oncontextmenu`
    // handler on the `.browser` root calls `menu.openAtCursor`
    // directly. The handler already exists from `-54` and stays
    // wired regardless of variant — fullstack-71 just relies on it
    // for dock-body right-click access.
    expect(fileBrowserSurface).toContain("oncontextmenu={onBrowserContextMenu}");
    expect(fileBrowserSurface).toContain(
      "menu?.openAtCursor(e.clientX, e.clientY)",
    );
  });

  test("dock variant has no on-surface unstick chrome button", () => {
    // The old dock chrome bar carried an unstick button next to the
    // kebab. With the bar gone, unstick is reachable only via:
    //   - "Unstick left/right" in the relocated right-click menu
    //   - Cmd+K < / > (from fullstack-69)
    // Drop the wrapper function + remove the unstick-button JSX.
    expect(fileBrowserSurface).not.toContain("function unstick()");
    expect(fileBrowserSurface).not.toContain('title={side === "right" ? "Unstick right" : "Unstick left"}');
  });
});
