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
    // `fullstack-a-68 slice 2` reshaped the Spawn group into a
    // Stage group: T / O / P / G / E stage additions into the
    // draft; Enter materializes; Esc discards. `v` no longer
    // surfaces in the cheatsheet (still aliased in the keymap
    // for muscle memory).
    expect(paneModeHelp).toContain('key: "ArrowUp"');
    expect(paneModeHelp).toContain('key: "ArrowLeft"');
    expect(paneModeHelp).toContain('key: "ArrowDown"');
    expect(paneModeHelp).toContain('key: "ArrowRight"');
    expect(paneModeHelp).toContain('key: "t"');
    expect(paneModeHelp).toContain('key: "o"');
    expect(paneModeHelp).toContain('key: "p"');
    expect(paneModeHelp).toContain('key: "g"');
    expect(paneModeHelp).toContain('key: "n"');
    expect(paneModeHelp).toContain('key: "Tab"');
    expect(paneModeHelp).toContain('key: "Escape"');
    expect(paneModeHelp).toContain('key: "Enter"');
    expect(paneModeHelp).toContain('key: "h"');
    // Numeric caps still gone (pre-`-a-32` cleanup).
    expect(paneModeHelp).not.toMatch(/key:\s*"1"/);
    expect(paneModeHelp).not.toMatch(/key:\s*"2"/);
    expect(paneModeHelp).not.toMatch(/key:\s*"3"/);
    expect(paneModeHelp).not.toMatch(/key:\s*"4"/);
  });

  test("fullstack-a-68 slice 2: spawn group renames to Stage (Enter to commit, Esc to discard)", () => {
    // Group title surfaces the transactional model; row labels
    // start with "Stage …".
    expect(paneModeHelp).toContain(
      'title: "Stage (Enter to commit, Esc to discard)"',
    );
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"t",\s*key:\s*"t"\s*\}\s*\],?\s*action:\s*"Stage Terminal"/,
    );
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"o",\s*key:\s*"o"\s*\}\s*\],?\s*action:\s*"Stage File Browser"/,
    );
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"p",\s*key:\s*"p"\s*\}\s*\],?\s*action:\s*"Stage Smart Prompt Terminal"/,
    );
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"g",\s*key:\s*"g"\s*\}\s*\],?\s*action:\s*"Stage Graph"/,
    );
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"n",\s*key:\s*"n"\s*\}\s*\],?\s*action:\s*"Stage New Draft"/,
    );
  });
});
