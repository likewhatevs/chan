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
    // `fullstack-a-32` reshaped the Spawn group: numeric 1/2/3/4
    // caps dropped (duplicated the new Cmd+T/O/P/Shift+M chord
    // family); letter mnemonics t/o/p/v are the universal in-
    // Hybrid-NAV path. Other groups (Move, Split, WASD, Commit)
    // unchanged.
    expect(paneModeHelp).toContain('key: "ArrowUp"');
    expect(paneModeHelp).toContain('key: "ArrowLeft"');
    expect(paneModeHelp).toContain('key: "ArrowDown"');
    expect(paneModeHelp).toContain('key: "ArrowRight"');
    expect(paneModeHelp).toContain('key: "t"');
    expect(paneModeHelp).toContain('key: "o"');
    expect(paneModeHelp).toContain('key: "p"');
    expect(paneModeHelp).toContain('key: "v"');
    expect(paneModeHelp).toContain('key: "Tab"');
    expect(paneModeHelp).toContain('key: "Escape"');
    expect(paneModeHelp).toContain('key: "Enter"');
    expect(paneModeHelp).toContain('key: "h"');
    // Numeric caps gone.
    expect(paneModeHelp).not.toMatch(/key:\s*"1"/);
    expect(paneModeHelp).not.toMatch(/key:\s*"2"/);
    expect(paneModeHelp).not.toMatch(/key:\s*"3"/);
    expect(paneModeHelp).not.toMatch(/key:\s*"4"/);
  });

  test("fullstack-a-32: spawn group uses letter mnemonics (t/o/p/v) only", () => {
    // Single-cap rows per spawn target; chord hint lives on the
    // top-level chord family (Cmd+T/O/P/Shift+M), not duplicated
    // in the cheatsheet.
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"t",\s*key:\s*"t"\s*\}\s*\],?\s*action:\s*"Spawn Terminal"/,
    );
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"o",\s*key:\s*"o"\s*\}\s*\],?\s*action:\s*"Spawn File Browser"/,
    );
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"p",\s*key:\s*"p"\s*\}\s*\],?\s*action:\s*"Spawn Rich Prompt"/,
    );
    expect(paneModeHelp).toMatch(
      /caps:\s*\[\s*\{\s*label:\s*"v",\s*key:\s*"v"\s*\}\s*\],?\s*action:\s*"Spawn Graph"/,
    );
  });
});
