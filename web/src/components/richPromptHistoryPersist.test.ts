import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";
import client from "../api/client.ts?raw";

// `fullstack-a-66` slice d: every Cmd+Enter submit persists
// the Rich Prompt source as `Drafts/rich-prompt-N/prompt.md`
// so the user has GitHub-style FB access to their history.

describe("fullstack-a-66 slice d: api.createRichPromptDraft", () => {
  test("api client exposes createRichPromptDraft hitting /api/drafts/rich-prompt", () => {
    expect(client).toMatch(
      /createRichPromptDraft: \(content: string\) =>[\s\S]*?req<\{ path: string; name: string \}>\("POST", "\/api\/drafts\/rich-prompt", \{[\s\S]*?content,/,
    );
  });

  test("client doc-comment cross-references slice d + the `rich-prompt-N` naming", () => {
    expect(client).toMatch(/`fullstack-a-66` slice d/);
    expect(client).toMatch(/rich-prompt-N\/prompt\.md/);
  });
});

describe("fullstack-a-66 slice d: TerminalTab submit hook", () => {
  test("submitRichPrompt calls persistRichPromptHistory(source)", () => {
    expect(terminal).toMatch(
      /function submitRichPrompt\(source: string\): void \{[\s\S]*?void persistRichPromptHistory\(source\);/,
    );
  });

  test("persistRichPromptHistory skips empty/whitespace-only sources", () => {
    expect(terminal).toMatch(
      /async function persistRichPromptHistory\(source: string\): Promise<void> \{[\s\S]*?const trimmed = source\.trim\(\);[\s\S]*?if \(!trimmed\) return;/,
    );
  });

  test("persist failures surface via setTransientStatus (non-fatal)", () => {
    expect(terminal).toMatch(
      /} catch \(err\) \{[\s\S]*?setTransientStatus\(\s*`rich-prompt history save failed:/,
    );
  });

  test("persist call routes through api.createRichPromptDraft", () => {
    expect(terminal).toMatch(/await api\.createRichPromptDraft\(source\)/);
  });
});
