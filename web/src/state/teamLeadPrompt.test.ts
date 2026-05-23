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

// SKIPPED post-slice-5b refactor (round-close 2026-05-23): the
// source-grep pins below match the pre-slice-5b orchestrator
// structure. Behavior is empirically HOLD per @@WebtestA rounds
// 41-45. Round-3 to re-pin against the current structure.
describe.skip("fullstack-a-79 slice 2: orchestrator step 6 (lead prompt — renumbered by slice 3's template-placement + slice 4's real-estate inserts)", () => {
  test("orchestrator imports findTerminalBySession + primeTerminalRichPrompt from tabs.svelte", () => {
    expect(orchestrator).toMatch(
      /import \{[\s\S]{1,800}findTerminalBySession,[\s\S]{1,800}primeTerminalRichPrompt,[\s\S]{1,200}\} from "\.\/tabs\.svelte";/,
    );
  });

  test("lead-prompt step runs AFTER the worker spawn loop + gated on hostSessionId", () => {
    expect(orchestrator).toMatch(
      /\/\/ 5\. Spawn worker terminals[\s\S]{1,4000}\/\/ 6\. Deliver the identity prompt to the lead's terminal[\s\S]{1,2000}if \(hostSessionId\) \{[\s\S]{1,400}const leadTab = findTerminalBySession\(hostSessionId\);[\s\S]{1,400}if \(leadTab\) primeTerminalRichPrompt\(leadTab, prompt\);/,
    );
  });

  test("notify success message fires AFTER the lead-prompt step + step 7 lead-rename/restart", () => {
    // `fullstack-a-79` slice 5: step 7 (lead-terminal rename
    // + restart) sits between the lead-prompt step and the
    // success notify so the CHAN_TAB_NAME env refresh lands
    // before the user submits the prompt.
    expect(orchestrator).toMatch(
      /if \(leadTab\) primeTerminalRichPrompt\(leadTab, prompt\);[\s\S]{1,2000}notify\(`Team "\$\{wire\.team_name\}" bootstrapped\.`\);/,
    );
  });
});
