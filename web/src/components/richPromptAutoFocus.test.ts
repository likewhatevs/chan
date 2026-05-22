import { describe, expect, test } from "vitest";
import richPrompt from "./TerminalRichPrompt.svelte?raw";
import tabsModule from "../state/tabs.svelte.ts?raw";

// fullstack-79: every `openActiveTerminalRichPrompt` call bumps a
// `focusNonce` on the rich-prompt state. TerminalRichPrompt's
// `$effect` watches the nonce and calls `wysiwygRef?.focusEnd()`
// (or `sourceRef?.focusAt(...)` in source mode) after a `tick()`
// to grab focus. Re-show via Cmd+K p / Cmd+P steals focus
// back even when `open` was already true.

describe("fullstack-79: rich-prompt auto-focus on entry", () => {
  test("TerminalRichPromptState declares a focusNonce field", () => {
    expect(tabsModule).toContain("focusNonce?: number");
  });

  test("openActiveTerminalRichPrompt bumps focusNonce on every call", () => {
    // The fresh-prompt branch seeds focusNonce: 1; the already-open
    // branch increments via `(focusNonce ?? 0) + 1` so a re-show
    // forces re-focus even when `open` was already true.
    expect(tabsModule).toContain("focusNonce: 1");
    expect(tabsModule).toContain(
      "tab.richPrompt.focusNonce = (tab.richPrompt.focusNonce ?? 0) + 1",
    );
  });

  test("TerminalRichPrompt focuses the editor when focusNonce changes", () => {
    // The reactive effect reads `prompt.focusNonce` to subscribe to
    // bumps, then dispatches to the appropriate editor child after
    // the next tick (lets the {#key mode()} block remount on
    // wysiwyg/source toggle before we try to focus).
    expect(richPrompt).toContain("void prompt.focusNonce");
    expect(richPrompt).toContain("wysiwygRef?.focusEnd()");
    expect(richPrompt).toContain("sourceRef?.focusAt(prompt.buffer.length)");
  });

  test("Source mode also receives focus via sourceRef binding", () => {
    expect(richPrompt).toContain("bind:this={sourceRef}");
  });
});
