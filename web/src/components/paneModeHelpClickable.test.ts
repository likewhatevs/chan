import { describe, expect, test } from "vitest";
import paneModeHelp from "./PaneModeHelp.svelte?raw";

// fullstack-63: every key-cap in the Hybrid NAV help overlay is a
// clickable button that dispatches a synthetic KeyboardEvent. The
// existing onWindowKey listener routes the event through the same
// `handlePaneModeKey` dispatcher that handles real keystrokes, so
// keyboard and mouse share one switch.

describe("fullstack-63: PaneModeHelp key-caps are clickable buttons", () => {
  test("dispatchKey synthesises a KeyboardEvent on the document", () => {
    expect(paneModeHelp).toContain("function dispatchKey(key: string): void");
    expect(paneModeHelp).toContain(
      'document.dispatchEvent(',
    );
    expect(paneModeHelp).toContain('new KeyboardEvent("keydown",');
  });

  test("clickable cap renders as <button> with kbd styling + dispatchKey onclick", () => {
    expect(paneModeHelp).toContain('class="kbd kbd-button"');
    expect(paneModeHelp).toContain("onclick={() => dispatchKey(cap.key!)}");
  });

  test("inert (descriptive-only) cap renders as <kbd> when cap.key is undefined", () => {
    // The Shift + [ ] - = row is the canonical inert cap — modifier
    // semantics can't be expressed as a single click, so the spec
    // says leave it descriptive-only.
    expect(paneModeHelp).toContain("Shift + [ ] - =");
    expect(paneModeHelp).toMatch(/{:else}\s*<kbd>{cap\.label}<\/kbd>\s*{\/if}/);
  });

  test("data carries the dispatch key for every clickable spawn / move / split cap", () => {
    // Spawn keys (1-4 + p / s) exit Pane Mode on commit; focus-move
    // arrows + split (/, \\) + WASD swap keep Pane Mode open. The
    // mapping lives in PaneModeHelp's `groups` data; spot-check the
    // key fields the spec calls out.
    expect(paneModeHelp).toContain('key: "ArrowUp"');
    expect(paneModeHelp).toContain('key: "ArrowLeft"');
    expect(paneModeHelp).toContain('key: "ArrowDown"');
    expect(paneModeHelp).toContain('key: "ArrowRight"');
    expect(paneModeHelp).toContain('key: "1"');
    expect(paneModeHelp).toContain('key: "2"');
    expect(paneModeHelp).toContain('key: "3"');
    expect(paneModeHelp).toContain('key: "4"');
    expect(paneModeHelp).toContain('key: "Tab"');
    expect(paneModeHelp).toContain('key: "Escape"');
    expect(paneModeHelp).toContain('key: "Enter"');
    expect(paneModeHelp).toContain('key: "h"');
  });
});
