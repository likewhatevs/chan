import { describe, expect, test } from "vitest";
import orchestrator from "./teamOrchestrator.svelte.ts?raw";

// `fullstack-a-79` slice 5: lead-terminal rename + PTY
// restart. The host's rich-prompt terminal IS the lead's
// terminal per addendum-b clarification #1; the orchestrator's
// step 6 stages the identity prompt in the rich-prompt buffer
// (which references `$CHAN_TAB_NAME` literally), and step 7
// renames the tab to the lead's handle + restarts the PTY so
// the shell's CHAN_TAB_NAME env-var actually equals the lead
// handle when the prompt submits.

describe("fullstack-a-79 slice 5: orchestrator step 7 (lead rename + restart)", () => {
  test("step 7 sits AFTER step 6 (lead prompt) and BEFORE the success notify", () => {
    expect(orchestrator).toMatch(
      /\/\/ 6\. Deliver the identity prompt[\s\S]{1,2000}\/\/ 7\. Rename \+ restart the host's terminal[\s\S]{1,2000}notify\(`Team "\$\{wire\.team_name\}" bootstrapped\.`\);/,
    );
  });

  test("gated on hostSessionId + leadTab + leadHandle + terminalSessionId", () => {
    expect(orchestrator).toMatch(
      /if \(hostSessionId\) \{[\s\S]{1,2000}const leadTab = findTerminalBySession\(hostSessionId\);[\s\S]{1,400}const leadEntry = wire\.members\.find\(\(m\) => m\.is_lead\);[\s\S]{1,400}const leadHandle = leadEntry\?\.handle;[\s\S]{1,400}if \(leadTab && leadHandle && leadTab\.terminalSessionId\) \{/,
    );
  });

  test("renames the tab via renameTerminalTab before the PTY restart", () => {
    expect(orchestrator).toMatch(
      /renameTerminalTab\(leadTab, leadHandle\);[\s\S]{1,400}await api\.restartTerminal\(/,
    );
  });

  test("api.restartTerminal carries the new name + the session windowId", () => {
    expect(orchestrator).toMatch(
      /api\.restartTerminal\(leadTab\.terminalSessionId, \{[\s\S]{1,400}name: leadHandle,[\s\S]{1,200}window_id: sessionWindowId\(\),[\s\S]{1,80}\}\);/,
    );
  });

  test("markTerminalEnvNameRestarted fires on success so the env-stale prompt clears", () => {
    expect(orchestrator).toMatch(
      /await api\.restartTerminal\([\s\S]{1,800}markTerminalEnvNameRestarted\(leadTab\);/,
    );
  });

  test("restart failure is non-fatal — surfaces via notify, does not bail the chain", () => {
    expect(orchestrator).toMatch(
      /try \{[\s\S]{1,400}await api\.restartTerminal\([\s\S]{1,1200}\} catch \(err\) \{[\s\S]{1,400}notify\([\s\S]{1,400}Lead terminal restart failed/,
    );
  });
});

describe("fullstack-a-79 slice 5: imports for step 7", () => {
  test("sessionWindowId imported from api/client", () => {
    expect(orchestrator).toMatch(
      /import \{ api, sessionWindowId,[\s\S]{1,200}\} from "\.\.\/api\/client";/,
    );
  });

  test("renameTerminalTab + markTerminalEnvNameRestarted imported from tabs.svelte", () => {
    expect(orchestrator).toMatch(/markTerminalEnvNameRestarted,/);
    expect(orchestrator).toMatch(/renameTerminalTab,/);
  });
});
