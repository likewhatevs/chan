import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";
import client from "../api/client.ts?raw";

// Phase 9: Rich Prompt submits archive through Core-owned
// workspaces instead of the legacy history-only draft endpoint.

describe("phase 9 rich prompt workspace API", () => {
  test("api client exposes the Core workspace routes", () => {
    expect(client).toMatch(
      /createRichPromptWorkspace: \(session: string, name\?: string\) =>[\s\S]*?"\/api\/rich-prompts"/,
    );
    expect(client).toMatch(/richPromptStatus: \(name: string, session\?: string\) =>/);
    expect(client).toMatch(/submitRichPromptWorkspace: \(/);
    expect(client).toMatch(/closeRichPromptWorkspace: \(name: string, session: string\) =>/);
  });

  test("status route passes the terminal session as query state", () => {
    expect(client).toMatch(
      /params\.set\("session", session\);[\s\S]*?\/api\/rich-prompts\/\$\{encodeURIComponent\(name\)\}\/status\$\{suffix\}/,
    );
  });
});

describe("phase 9 rich prompt submit hook", () => {
  test("submitRichPrompt archives through persistRichPromptSubmission(source)", () => {
    expect(terminal).toMatch(
      /function submitRichPrompt\(source: string\): void \{[\s\S]*?void persistRichPromptSubmission\(source\);/,
    );
  });

  test("shell mode appends a missing newline before sending to the PTY", () => {
    expect(terminal).toMatch(
      /sendUserInput\(source\.endsWith\("\\n"\) \? source : `\$\{source\}\\n`\);/,
    );
  });

  test("persistRichPromptSubmission skips empty/whitespace-only sources", () => {
    expect(terminal).toMatch(
      /async function persistRichPromptSubmission\(source: string\): Promise<void> \{[\s\S]*?const trimmed = source\.trim\(\);[\s\S]*?if \(!trimmed\) return;/,
    );
  });

  test("persist call routes through api.submitRichPromptWorkspace", () => {
    expect(terminal).toMatch(
      /await api\.submitRichPromptWorkspace\(name, \{[\s\S]*?content: source,[\s\S]*?expected_sequence: rp\.submissionSequence \?\? 0,/,
    );
  });

  test("terminal close sink routes Rich Prompt teardown through Core close", () => {
    expect(terminal).toMatch(
      /registerTerminalCloseSink\(tab\.id, closeTerminalForTab\)/,
    );
    expect(terminal).toMatch(/await api\.closeRichPromptWorkspace\(name, tab\.terminalSessionId \?\? ""\)/);
  });
});
