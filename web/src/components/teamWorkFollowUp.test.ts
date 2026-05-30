import { describe, expect, test } from "vitest";
import bubble from "./BubbleOverlay.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";

// BubbleOverlay is now a frontend-only static example. The F-follow-up
// feature (surveyAsQuoteMarkdown / quoteSurveyToPrompt quoting a live
// survey into the team-work buffer) is gone along with the watcher data
// it depended on. The "F" affordance survives as a presentational
// example marker; clicking it dismisses the static example.

describe("Team Work revamp: F follow-up is presentational only", () => {
  test("the F-key handler / quote-to-prompt plumbing is removed from BubbleOverlay", () => {
    // No survey-to-markdown quoting helper.
    expect(bubble).not.toMatch(/function surveyAsQuoteMarkdown\b/);
    expect(bubble).not.toMatch(/function quoteSurveyToPrompt\b/);
    // No window keydown handler quoting on F.
    expect(bubble).not.toMatch(/quoteSurveyToPrompt\(/);
    // No onQuoteToPrompt prop / callback.
    expect(bubble).not.toMatch(/onQuoteToPrompt/);
  });

  test("the static follow-up affordance renders with an F marker", () => {
    expect(bubble).toMatch(/class="follow-button"/);
    expect(bubble).toMatch(/<kbd>F<\/kbd>/);
    // It is wired to the generic dismiss handler, not a reply path.
    expect(bubble).toMatch(/class="follow-button" onclick=\{dismiss\}/);
  });

  test("TerminalTab no longer carries the quoteIntoTeamWork callback", () => {
    expect(terminal).not.toMatch(/quoteIntoTeamWork/);
    expect(terminal).not.toMatch(/onQuoteToPrompt/);
  });

  test("the BubbleOverlay mount in TerminalTab passes no watcher props", () => {
    // Self-contained overlay: mounted with no props at all.
    expect(terminal).toMatch(/<BubbleOverlay \/>/);
    expect(terminal).not.toMatch(/watcher=\{tab\.watcher\}/);
  });
});
