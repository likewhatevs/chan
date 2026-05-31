import { describe, expect, test } from "vitest";
import bubble from "./BubbleOverlay.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";

// BubbleOverlay is a Wave-1 placeholder that renders nothing; the real
// reply-capable survey overlay is rebuilt in Wave 2. The old F-follow-up
// feature (surveyAsQuoteMarkdown / quoteSurveyToPrompt quoting a live
// survey into the team-work buffer) is gone along with the watcher data
// it depended on, and the gutted placeholder carries no example markup.

describe("Team Work revamp: survey-quote plumbing stays removed", () => {
  test("the F-key handler / quote-to-prompt plumbing is removed from BubbleOverlay", () => {
    // No survey-to-markdown quoting helper.
    expect(bubble).not.toMatch(/function surveyAsQuoteMarkdown\b/);
    expect(bubble).not.toMatch(/function quoteSurveyToPrompt\b/);
    // No window keydown handler quoting on F.
    expect(bubble).not.toMatch(/quoteSurveyToPrompt\(/);
    // No onQuoteToPrompt prop / callback.
    expect(bubble).not.toMatch(/onQuoteToPrompt/);
  });

  test("TerminalTab no longer carries the quoteIntoTeamWork callback", () => {
    expect(terminal).not.toMatch(/quoteIntoTeamWork/);
    expect(terminal).not.toMatch(/onQuoteToPrompt/);
  });

  test("the BubbleOverlay mount in TerminalTab passes no watcher props", () => {
    // Self-contained placeholder: mounted with no props at all.
    expect(terminal).toMatch(/<BubbleOverlay \/>/);
    expect(terminal).not.toMatch(/watcher=\{tab\.watcher\}/);
  });
});
