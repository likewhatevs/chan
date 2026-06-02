import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";

// Rich Prompt - the terminal wiring (slice 2): TerminalTab registers the prompt
// sink (WS `prompt` frame, NOT raw input), mounts the bubble over the active
// terminal, and exposes the right-click "Show/Hide Rich Prompt" entry. The
// bubble component + toggle + send seam are covered in
// richPromptComponent.test.ts. Real interaction is browser-smoked.

describe("TerminalTab Rich Prompt wiring", () => {
  test("registers a prompt sink that sends the `prompt` frame (not raw input)", () => {
    expect(terminal).toMatch(/registerTerminalPromptSink\(tab\.id, sendPrompt\)/);
    expect(terminal).toMatch(
      /function sendPrompt\(data: string, agent\?: string\): void \{[\s\S]{1,200}send\(\{ type: "prompt", data, \.\.\.\(agent \? \{ agent \} : \{\}\) \}\)/,
    );
  });

  test("unregisters the prompt sink on teardown", () => {
    expect(terminal).toMatch(
      /const unregisterPrompt = registerTerminalPromptSink[\s\S]{1,400}unregisterPrompt\(\)/,
    );
  });

  test("mounts <RichPrompt> only on the active terminal when visible, passing the tab", () => {
    expect(terminal).toMatch(/import RichPrompt from "\.\/RichPrompt\.svelte"/);
    // The tab is passed so the bubble binds to THIS terminal's per-terminal
    // Drafts-backed draft.
    expect(terminal).toMatch(
      /\{#if active && richPrompt\.visible\}[\s\S]{1,80}<RichPrompt \{tab\} \/>/,
    );
  });

  test("discards the per-terminal Rich Prompt draft folder on terminal close", () => {
    // @@Host lifecycle: the draft (draft.md + pasted media) is tied to the
    // terminal; closing the terminal deletes the whole folder so nothing leaks.
    expect(terminal).toMatch(
      /function closeTerminalForTab\(\): boolean \{[\s\S]{1,400}if \(tab\.richPromptDraftPath\) \{[\s\S]{1,120}api\.discardDraft\(tab\.richPromptDraftPath\)/,
    );
  });

  test("right-click menu has a Show/Hide Rich Prompt entry with the chord", () => {
    expect(terminal).toMatch(
      /onclick=\{toggleRichPromptFromMenu\}[\s\S]{1,260}richPrompt\.visible \? "Hide Rich Prompt" : "Show Rich Prompt"[\s\S]{1,120}\{richPromptChord\}/,
    );
    expect(terminal).toMatch(
      /const richPromptChord = formatChord\("Mod\+Shift\+P", currentOS\(\)\)/,
    );
  });
});
