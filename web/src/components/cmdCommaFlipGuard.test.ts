import { describe, expect, test } from "vitest";
import appSource from "../App.svelte?raw";

// Cmd+, flips the focused Hybrid pane. It must NOT fire while a modal or
// the search overlay owns the keyboard, or it flips the pane hidden
// behind the surface - the reported "panes flip" desync. Reproduced on a
// real build: open Search (or the New file dialog), press Cmd+,, dismiss
// the surface -> the obscured pane had silently flipped to its back.
//
// Both Cmd+, paths route through the same flip and must share the guard:
// the web chord (onWindowKey) and chan-desktop's KEY_BRIDGE_JS, which
// replays the native Cmd+, as the `app.settings.toggle` command. The
// behavior is verified in the browser; these lock the wiring.
describe("Cmd+, pane-flip modal/overlay guard", () => {
  const src = appSource.replace(/\s+/g, " ");

  test("paneChordBlocked checks the overlay stack and every modal", () => {
    expect(src).toContain(
      "function paneChordBlocked(): boolean { return ( topOverlay() !== null " +
        "|| promptState.open || pathPromptState.open || confirmState.open " +
        "|| draftCloseState.open );",
    );
  });

  test("both Cmd+, flip entry points bail when a modal or overlay is active", () => {
    const guards = src.match(/if \(paneChordBlocked\(\)\) return;/g) ?? [];
    expect(guards.length).toBeGreaterThanOrEqual(2);
  });

  test("the flip action is preserved at both entry points", () => {
    const flips = src.match(/flipHybrid\(layout\.activePaneId\)/g) ?? [];
    expect(flips.length).toBeGreaterThanOrEqual(2);
  });
});
