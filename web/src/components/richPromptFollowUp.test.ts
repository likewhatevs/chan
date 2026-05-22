import { describe, expect, test } from "vitest";
import bubble from "./BubbleOverlay.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";

// `fullstack-a-69`: F-follow-up rewrite. Pressing F (or clicking
// the F-follow-up affordance) brings the current survey into the
// Rich Prompt as a markdown quote + lands the caret on the new
// line below.

describe("fullstack-a-69: BubbleOverlay surveyAsQuoteMarkdown helper", () => {
  test("helper exists + formats topic + from + questions as `> `-prefixed lines", () => {
    expect(bubble).toMatch(
      /function surveyAsQuoteMarkdown\(event: WatcherEvent\): string \{[\s\S]*?lines\.push\(`> \*\*\$\{event\.topic\}\*\*`\)/,
    );
    expect(bubble).toMatch(/lines\.push\(`> \*\*\$\{q\.header\}\*\*`\)/);
    expect(bubble).toMatch(/lines\.push\(`> \$\{q\.text\}`\)/);
    expect(bubble).toMatch(/lines\.push\(`>   - \$\{opt\.key\}: \$\{opt\.label\}`\)/);
  });

  test("helper falls back to event.note when no questions present", () => {
    expect(bubble).toMatch(
      /if \(qs\.length === 0 && event\.note\) \{[\s\S]*?lines\.push\(`> \$\{event\.note\}`\)/,
    );
  });
});

describe("fullstack-a-69: BubbleOverlay F-key + button wired to quoteSurveyToPrompt", () => {
  test("F-key handler calls quoteSurveyToPrompt (not the removed markFollowUp)", () => {
    expect(bubble).toMatch(
      /if \(e\.key === "f" \|\| e\.key === "F"\) \{[\s\S]*?quoteSurveyToPrompt\(event\);/,
    );
  });

  test("follow-up button onclick calls quoteSurveyToPrompt", () => {
    expect(bubble).toMatch(
      /class="follow-button"[\s\S]*?onclick=\{\(\) => quoteSurveyToPrompt\(event\)\}/,
    );
  });

  test("markFollowUp function definition removed (no orphan)", () => {
    expect(bubble).not.toMatch(/async function markFollowUp\b/);
    expect(bubble).not.toMatch(/function markFollowUp\b/);
  });

  test("onQuoteToPrompt prop accepted on BubbleOverlay", () => {
    expect(bubble).toMatch(/onQuoteToPrompt\?: \(markdown: string\) => void;/);
  });
});

describe("fullstack-a-69: TerminalTab provides quoteIntoRichPrompt callback", () => {
  test("BubbleOverlay mount passes onQuoteToPrompt callback", () => {
    expect(terminal).toMatch(
      /onQuoteToPrompt=\{\(markdown\) => quoteIntoRichPrompt\(markdown\)\}/,
    );
  });

  test("quoteIntoRichPrompt appends to buffer + opens prompt + bumps focusNonce", () => {
    expect(terminal).toMatch(
      /function quoteIntoRichPrompt\(markdown: string\): void \{[\s\S]*?rp\.buffer = `\$\{rp\.buffer\}\$\{separator\}\$\{markdown\}\\n`;[\s\S]*?rp\.open = true;[\s\S]*?rp\.focusNonce = \(rp\.focusNonce \?\? 0\) \+ 1;/,
    );
  });

  test("separator added between existing buffer + new quote (no clobber)", () => {
    expect(terminal).toMatch(
      /const separator = rp\.buffer\.length === 0 \? "" : "\\n\\n";/,
    );
  });
});
