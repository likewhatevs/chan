import { describe, expect, test } from "vitest";
import bubble from "./BubbleOverlay.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";
import app from "../App.svelte?raw";

// The old F-follow-up feature (surveyAsQuoteMarkdown / quoteSurveyToPrompt
// quoting a live survey into the team-work buffer) is gone along with the
// watcher data it depended on. The rebuilt survey overlay (BubbleOverlay)
// replies through POST /api/survey/reply instead, and is mounted once at the
// App root rather than per terminal tab.

describe("Team Work revamp: survey-quote plumbing stays removed", () => {
  test("the old quote-to-prompt plumbing is absent from BubbleOverlay", () => {
    // No survey-to-markdown quoting helper.
    expect(bubble).not.toMatch(/function surveyAsQuoteMarkdown\b/);
    expect(bubble).not.toMatch(/function quoteSurveyToPrompt\b/);
    expect(bubble).not.toMatch(/quoteSurveyToPrompt\(/);
    // No onQuoteToPrompt prop / callback.
    expect(bubble).not.toMatch(/onQuoteToPrompt/);
  });

  test("the rebuilt overlay replies via the survey store, not a quote prop", () => {
    // The reply round-trip lives in the survey store (pickOption /
    // requestFollowup); the overlay carries no quote-into-prompt callback.
    expect(bubble).toMatch(/from "\.\.\/state\/survey\.svelte"/);
    expect(bubble).toMatch(/pickOption|requestFollowup/);
  });

  test("TerminalTab no longer carries the quoteIntoTeamWork callback", () => {
    expect(terminal).not.toMatch(/quoteIntoTeamWork/);
    expect(terminal).not.toMatch(/onQuoteToPrompt/);
  });

  test("the survey overlay is mounted once at App root, not per terminal tab", () => {
    // Window-level modal: a single mount at App root, none per terminal tab.
    expect(app).toMatch(/<BubbleOverlay \/>/);
    expect(terminal).not.toMatch(/BubbleOverlay/);
  });
});
