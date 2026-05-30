import { describe, expect, test } from "vitest";
import teamWork from "./TeamWork.svelte?raw";

// Cursor/placeholder Y-alignment. Tests pin the CSS rule + the
// leading-space placeholder text (`{cursor}{space}{default-text}`).
// Behavioral verification (sub-pixel cursor + placeholder alignment,
// visible blink at the start of the prompt) needs a browser; this
// suite only pins the source shape.

describe("leading space in PROMPT_PLACEHOLDER_TEXT", () => {
  test("constant starts with a single space character", () => {
    // The placeholder copy advertises the Enter / Shift+Enter chord
    // split. The leading single space is load-bearing
    // (`{cursor}{space}{default-text}`).
    expect(teamWork).toMatch(
      /const PROMPT_PLACEHOLDER_TEXT = " Write your prompt; Enter to send, Shift\+Enter for a new line";/,
    );
  });

  test("no double-space at the start (regression guard)", () => {
    expect(teamWork).not.toMatch(
      /const PROMPT_PLACEHOLDER_TEXT = "  /,
    );
  });
});

describe("scoped .cm-placeholder CSS override", () => {
  test("rule scoped to .team-work so it doesn't leak to other CM6 editors", () => {
    expect(teamWork).toMatch(
      /:global\(\.team-work \.cm-placeholder\) \{/,
    );
  });

  test("line-height: 1.2 (matches CM6's font-natural line-box)", () => {
    expect(teamWork).toMatch(
      /:global\(\.team-work \.cm-placeholder\) \{[\s\S]{1,400}line-height: 1\.2;/,
    );
  });

  test("vertical-align: middle (centers placeholder text on cursor's Y midpoint)", () => {
    expect(teamWork).toMatch(
      /:global\(\.team-work \.cm-placeholder\) \{[\s\S]{1,400}vertical-align: middle;/,
    );
  });

  test("CSS comment records the root-cause analysis", () => {
    // The comment names the mechanism (CM6 sizes the cursor from the
    // font; the placeholder inherits line-height 1.8) and the fix
    // shape (collapse to the cursor's natural line-box + center).
    expect(teamWork).toMatch(
      /CM6 sizes the[\s\S]{1,80}cursor from the font's natural line-box/i,
    );
    expect(teamWork).toMatch(/placeholder span inherits[\s\S]{1,80}line-height/i);
  });
});
