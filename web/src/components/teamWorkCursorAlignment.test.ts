import { describe, expect, test } from "vitest";
import teamWork from "./TeamWork.svelte?raw";

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
    // Phase-13 bug 4: copy updated to advertise the chat-style
    // Enter / Shift+Enter chord split. Leading single space is
    // still load-bearing per `{cursor}{space}{default-text}`.
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

describe("fullstack-a-89b: scoped .cm-placeholder CSS override", () => {
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

  test("CSS comment records the empirical measurements + root-cause analysis", () => {
    // Future readers should be able to trace the WHY back to
    // the saga without diving into git log. The comment block
    // names the metric (cursor 19px vs line 28.8px), the
    // mechanism (CM6 sizes cursor from font; placeholder
    // inherits line-height), and the fix shape.
    expect(teamWork).toMatch(
      /`fullstack-a-89b`: empirical fix for the cursor\/placeholder/,
    );
    expect(teamWork).toMatch(/cursor.*top 717\.5.*bottom 736\.5.*height 19/);
    expect(teamWork).toMatch(
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
    expect(teamWork).toMatch(/-a-24/);
    expect(teamWork).toMatch(/-a-84/);
    expect(teamWork).toMatch(/-a-87/);
    expect(teamWork).toMatch(/`fullstack-a-89`/);
  });
});
