import { describe, expect, test } from "vitest";
import fileBrowserSurface from "./FileBrowserSurface.svelte?raw";

// Tab and dock variants both dropped the surface `<header>`. FB-specific
// menu items are served via a triggerless HamburgerMenu opened at the
// cursor by the tab-strip right-click or by `onBrowserContextMenu` on
// the dock body. Only the Overlay variant keeps a header (close +
// maximize + kebab - maximize has nowhere else to live).

describe("FileBrowserSurface header is overlay-only", () => {
  test("header is wrapped in {#if isOverlay}", () => {
    expect(fileBrowserSurface).toMatch(/{\s*#if\s+isOverlay\s*}[\s\S]*?<header>/);
  });

  test("non-overlay variants (tab + dock) render a triggerless HamburgerMenu instead of the header", () => {
    // The {:else} branch mounts a HamburgerMenu with showTrigger={false}
    // so FB-specific items are reachable via openAtCursor without
    // painting a visible trigger.
    expect(fileBrowserSurface).toMatch(
      /{\s*:else\s*}[\s\S]*?<HamburgerMenu[\s\S]*?showTrigger=\{false\}/,
    );
  });

  test("an $effect mirrors tab-strip right-click into menu.openAtCursor", () => {
    // Pane.svelte calls openTabMenu(t.id, anchor) on tab right-click;
    // this effect mirrors that into menu.openAtCursor() so FB-specific
    // items render on the tab variant.
    expect(fileBrowserSurface).toContain("tabMenu.openForTabId");
    expect(fileBrowserSurface).toContain("tabMenu.anchor");
    expect(fileBrowserSurface).toContain("menu?.openAtCursor(anchor.left, anchor.top)");
  });

  test("dock-body right-click flows through onBrowserContextMenu → menu.openAtCursor", () => {
    // Dock variant has no tab strip; the dock-body `oncontextmenu`
    // handler on the `.browser` root calls `menu.openAtCursor`
    // directly; the handler is wired regardless of variant so the
    // dock-body right-click stays accessible.
    expect(fileBrowserSurface).toContain("oncontextmenu={onBrowserContextMenu}");
    expect(fileBrowserSurface).toContain(
      "menu?.openAtCursor(e.clientX, e.clientY)",
    );
  });

  test("dock variant has no on-surface unstick chrome button", () => {
    // With the dock chrome bar gone, unstick is reachable only via
    // the right-click menu or Cmd+K < / >.
    expect(fileBrowserSurface).not.toContain("function unstick()");
    expect(fileBrowserSurface).not.toContain('title={side === "right" ? "Unstick right" : "Unstick left"}');
  });
});
