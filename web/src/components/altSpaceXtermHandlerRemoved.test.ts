import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";

// `fullstack-a-94` follow-up to `-a-90`: webtest-a caught a
// third Alt+Space handler in TerminalTab.svelte's
// `handleTerminalKeyEvent` (xterm `customKeyEventHandler`
// translation layer). `-a-90`'s grep covered the two
// keymap-driven branches but missed this one. Remove + pin.

describe("fullstack-a-94: 3rd Alt+Space handler removed from handleTerminalKeyEvent", () => {
  test("no Alt+Space branch inside handleTerminalKeyEvent", () => {
    expect(terminal).not.toMatch(
      /function handleTerminalKeyEvent\(e: KeyboardEvent\): boolean \{[\s\S]*?e\.altKey &&[\s\S]*?e\.code === "Space"/,
    );
  });

  test("handleTerminalKeyEvent still defers to handleTerminalMetaKey", () => {
    // Sanity check: removal shouldn't strip the meta-key
    // delegation that handles the legitimate terminal-side
    // keymap surfaces (Cmd+K leader chord, etc.).
    expect(terminal).toMatch(
      /function handleTerminalKeyEvent\(e: KeyboardEvent\): boolean \{[\s\S]*?return handleTerminalMetaKey\(e, sendUserInput\);/,
    );
  });

  test("removal-rationale comment cites the audit-grep miss + the empirical catch", () => {
    expect(terminal).toMatch(
      /`fullstack-a-94`:[\s\S]*?third Alt\+Space handler[\s\S]*?customKeyEventHandler/i,
    );
  });

  test("attachCustomKeyEventHandler registration still present (untouched mechanism)", () => {
    // Sanity: the registration line at ~424 stays; we only
    // removed the chord branch inside the registered handler.
    expect(terminal).toMatch(
      /term\.attachCustomKeyEventHandler\(handleTerminalKeyEvent\)/,
    );
  });
});
