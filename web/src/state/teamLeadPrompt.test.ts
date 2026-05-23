import { describe, expect, test } from "vitest";
import orchestrator from "./teamOrchestrator.svelte.ts?raw";
import tabs from "./tabs.svelte.ts?raw";

// `fullstack-a-79` slice 2: deliver the identity prompt to
// the lead's terminal (the host session) via the rich-prompt
// buffer. Tests pin the architectural shape of the new
// helpers + the orchestrator's step-4 addition.

describe("fullstack-a-79 slice 2: tabs.svelte helpers", () => {
  test("findTerminalBySession walks allTerminalTabs + matches on terminalSessionId", () => {
    expect(tabs).toMatch(
      /export function findTerminalBySession\(sessionId: string\): TerminalTab \| null \{[\s\S]{1,800}for \(const tab of allTerminalTabs\(\)\) \{[\s\S]{1,200}if \(tab\.terminalSessionId === sessionId\) return tab;/,
    );
  });

  test("primeTerminalRichPrompt initializes the buffer + flags open + opts default mode", () => {
    expect(tabs).toMatch(
      /export function primeTerminalRichPrompt\(tab: TerminalTab, text: string\): void \{[\s\S]{1,1000}tab\.richPrompt = \{[\s\S]{1,400}buffer: text,[\s\S]{1,200}open: true,[\s\S]{1,200}mode: "wysiwyg",/,
    );
  });

  test("primeTerminalRichPrompt's already-armed branch overwrites buffer + flips open", () => {
    expect(tabs).toMatch(
      /tab\.richPrompt\.buffer = text;[\s\S]{1,200}tab\.richPrompt\.open = true;[\s\S]{1,200}tab\.richPrompt\.mode \?\?= "wysiwyg";/,
    );
  });
});

describe("fullstack-a-79 slice 2: orchestrator step 5 (lead prompt — renumbered by slice 3's template-placement insert)", () => {
  test("orchestrator imports the two new helpers from tabs.svelte", () => {
    expect(orchestrator).toMatch(
      /import \{[\s\S]{1,400}findTerminalBySession,[\s\S]{1,200}openTerminalInActivePane,[\s\S]{1,200}primeTerminalRichPrompt,[\s\S]{1,80}\} from "\.\/tabs\.svelte";/,
    );
  });

  test("lead-prompt step runs AFTER the worker spawn loop + gated on hostSessionId", () => {
    expect(orchestrator).toMatch(
      /\/\/ 4\. Spawn worker terminals[\s\S]{1,4000}\/\/ 5\. Deliver the identity prompt to the lead's terminal[\s\S]{1,2000}if \(hostSessionId\) \{[\s\S]{1,400}const leadTab = findTerminalBySession\(hostSessionId\);[\s\S]{1,400}if \(leadTab\) primeTerminalRichPrompt\(leadTab, prompt\);/,
    );
  });

  test("notify success message fires AFTER the lead-prompt step", () => {
    expect(orchestrator).toMatch(
      /if \(leadTab\) primeTerminalRichPrompt\(leadTab, prompt\);[\s\S]{1,400}notify\(`Team "\$\{wire\.team_name\}" bootstrapped\.`\);/,
    );
  });
});
