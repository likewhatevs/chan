import { describe, expect, test } from "vitest";
import richPrompt from "./TerminalRichPrompt.svelte?raw";

// `fullstack-a-89b`: empirical cursor/placeholder Y-alignment
// fix. Tests pin the CSS rule + the leading-space placeholder
// text per @@Alex's literal `{cursor}{space}{default-text}`
// spec. Behavioral verification (sub-pixel cursor + placeholder
// alignment, visible blink at the start of the prompt) lives
// in the browser-empirical walk @@WebtestA performs against
// the running binary.
//
// Pre-fix measurements (default CM6 baseTheme):
//   cursor:      top 717.5, bottom 736.5, height 19
//   placeholder: top 713.0, bottom 741.8, height 28.8
//   delta:       +4.5px top, -5.3px bottom (cursor floats
//                4.5px above placeholder text top)
//
// Post-fix measurements (line-height 1.2 + vertical-align middle):
//   cursor:      top 719.29, bottom 738.29, height 19
//   placeholder: top 719.19, bottom 738.38, height 19.20
//   delta:       +0.10px top, -0.09px bottom (sub-pixel alignment)

describe("fullstack-a-89b: leading space in PROMPT_PLACEHOLDER_TEXT", () => {
  test("constant starts with a single space character", () => {
    expect(richPrompt).toMatch(
      /const PROMPT_PLACEHOLDER_TEXT = " Write a multi-line command and Cmd\+Enter";/,
    );
  });

  test("no double-space at the start (regression guard)", () => {
    expect(richPrompt).not.toMatch(
      /const PROMPT_PLACEHOLDER_TEXT = "  /,
    );
  });
});

describe("fullstack-a-89b: scoped .cm-placeholder CSS override", () => {
  test("rule scoped to .rich-prompt so it doesn't leak to other CM6 editors", () => {
    expect(richPrompt).toMatch(
      /:global\(\.rich-prompt \.cm-placeholder\) \{/,
    );
  });

  test("line-height: 1.2 (matches CM6's font-natural line-box)", () => {
    expect(richPrompt).toMatch(
      /:global\(\.rich-prompt \.cm-placeholder\) \{[\s\S]{1,400}line-height: 1\.2;/,
    );
  });

  test("vertical-align: middle (centers placeholder text on cursor's Y midpoint)", () => {
    expect(richPrompt).toMatch(
      /:global\(\.rich-prompt \.cm-placeholder\) \{[\s\S]{1,400}vertical-align: middle;/,
    );
  });

  test("CSS comment records the empirical measurements + root-cause analysis", () => {
    // Future readers should be able to trace the WHY back to
    // the saga without diving into git log. The comment block
    // names the metric (cursor 19px vs line 28.8px), the
    // mechanism (CM6 sizes cursor from font; placeholder
    // inherits line-height), and the fix shape.
    expect(richPrompt).toMatch(
      /`fullstack-a-89b`: empirical fix for the cursor\/placeholder/,
    );
    expect(richPrompt).toMatch(/cursor.*top 717\.5.*bottom 736\.5.*height 19/);
    expect(richPrompt).toMatch(
      /placeholder default: top 713, bottom 741\.8, height 28\.8/,
    );
  });
});

describe("fullstack-a-89b: the -a-89 saga history stays documented", () => {
  test("comment cites the prior fix chain (-a-24 / -a-84 / -a-87 / -a-89)", () => {
    // The comment block above the new rule retains the
    // breadcrumbs to the prior attempts; future debugging
    // should land on the empirical-first directive +
    // architect's note that this was a 3rd-round UX bug.
    expect(richPrompt).toMatch(/-a-24/);
    expect(richPrompt).toMatch(/-a-84/);
    expect(richPrompt).toMatch(/-a-87/);
    expect(richPrompt).toMatch(/`fullstack-a-89`/);
  });
});
