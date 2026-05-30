import { describe, expect, test } from "vitest";
import teamWork from "./TeamWork.svelte?raw";
import tabsModule from "../state/tabs.svelte.ts?raw";

// Every `openActiveTeamWork` call bumps a `focusNonce` on the Team
// Work state. The `$effect` watches the nonce and calls
// `wysiwygRef?.focusEnd()` (or `sourceRef?.focusAt(...)` in source
// mode) after a `tick()`. Re-show via Cmd+K p / Cmd+P steals focus
// back even when `open` was already true.

describe("Team Work auto-focus on entry", () => {
  test("TeamWorkState declares a focusNonce field", () => {
    expect(tabsModule).toContain("focusNonce?: number");
  });

  test("openActiveTeamWork bumps focusNonce on every call", () => {
    // The fresh-prompt branch seeds focusNonce: 1; the already-open
    // branch increments so a re-show forces re-focus.
    expect(tabsModule).toContain("focusNonce: 1");
    expect(tabsModule).toContain(
      "tab.teamWork.focusNonce = (tab.teamWork.focusNonce ?? 0) + 1",
    );
  });

  test("TeamWork focuses the editor when focusNonce changes", () => {
    // The reactive effect reads `prompt.focusNonce`, then dispatches to
    // the appropriate editor child after the next tick so the
    // {#key mode()} block has time to remount on mode toggle.
    expect(teamWork).toContain("void prompt.focusNonce");
    expect(teamWork).toContain("wysiwygRef?.focusEnd()");
    expect(teamWork).toContain("sourceRef?.focusAt(prompt.buffer.length)");
  });

  test("Source mode also receives focus via sourceRef binding", () => {
    expect(teamWork).toContain("bind:this={sourceRef}");
  });
});
