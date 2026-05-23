import { describe, expect, test } from "vitest";
import richPrompt from "./TerminalRichPrompt.svelte?raw";
import orchestrator from "../state/teamOrchestrator.svelte.ts?raw";
import client from "../api/client.ts?raw";

// `fullstack-a-79` slice 1: integration pins covering the
// dialog → orchestrator handoff + the chan-server API wiring
// for the team workspace endpoints.

describe("fullstack-a-79 slice 1: TerminalRichPrompt onBootstrap dispatches the orchestrator", () => {
  test("openNewTeamDialog passes onBootstrap → runTeamBootstrap", () => {
    expect(richPrompt).toMatch(
      /openGlobalTeamDialog\(\{[\s\S]{1,800}onBootstrap: async \(config\) => \{[\s\S]{1,400}await runTeamBootstrap\(config, terminalSessionId\);/,
    );
  });

  test("runTeamBootstrap imported from state/teamOrchestrator.svelte", () => {
    expect(richPrompt).toMatch(
      /import \{ runTeamBootstrap \} from "\.\.\/state\/teamOrchestrator\.svelte";/,
    );
  });
});

describe("fullstack-a-79 slice 1: orchestrator bootstrap chain", () => {
  test("runTeamBootstrap walks teamCreate → placeTeamTemplates → teamLoad → spawnTerminal per worker", () => {
    expect(orchestrator).toMatch(
      /export async function runTeamBootstrap\([\s\S]{1,400}\): Promise<void> \{[\s\S]{1,4000}await api\.teamCreate\(wire\.team_name, wire\);[\s\S]{1,2000}await placeTeamTemplates\(wire\);[\s\S]{1,2000}await api\.teamLoad\(wire\.team_name\);[\s\S]{1,2000}await api\.spawnTerminal\(/,
    );
  });

  test("lead member is skipped from spawn loop (host session is the lead's terminal)", () => {
    // `fullstack-a-79` slice 4: loop now uses an indexed walk
    // (`for (let i = 0; …)`) to look up the member's assigned
    // pane id from the resolved real-estate map. The lead skip
    // stays — host session IS the lead's terminal per
    // addendum-b clarification #1.
    expect(orchestrator).toMatch(
      /for \(let i = 0; i < wire\.members\.length; i \+= 1\) \{[\s\S]{1,800}if \(m\.is_lead\) continue;/,
    );
  });

  test("each worker terminal opens in its resolved pane (cell from split, or active pane for tabs) with the identity prompt as seedInput", () => {
    // `fullstack-a-79` slice 4: split-pane real estate honored.
    // openTerminalInPane(paneId, …) replaces
    // openTerminalInActivePane so workers land in their
    // assigned cells.
    expect(orchestrator).toMatch(
      /const paneId = memberPaneIds\.workers\[i\] \?\? layout\.activePaneId;[\s\S]{1,400}openTerminalInPane\(paneId, \{[\s\S]{1,400}sessionId: response\.session,[\s\S]{1,200}title: response\.tab_label,[\s\S]{1,200}seedInput: prompt,/,
    );
  });

  test("split-pane real estate now WIRED (slice 4 — was scope-poked in slice 1)", () => {
    // The slice-1 notify("Split-pane real estate not yet
    // wired") is gone now that the orchestrator builds the
    // grid via buildSplitGrid + maps members through
    // resolveMemberPaneIds.
    expect(orchestrator).not.toMatch(
      /Split-pane real estate not yet wired/,
    );
    expect(orchestrator).toMatch(
      /const memberPaneIds = resolveMemberPaneIds\(config\);/,
    );
  });

  test("focus restored to the lead's pane after the spawn loop", () => {
    expect(orchestrator).toMatch(
      /if \(leadPaneId\) setActivePane\(leadPaneId\);/,
    );
  });
});

describe("fullstack-a-79 slice 1: api client team endpoints", () => {
  test("teamCreate POSTs /api/teams with { name, config }", () => {
    expect(client).toMatch(
      /teamCreate: \(name: string, config: TeamConfigWire\) =>[\s\S]{1,200}req<TeamRefView>\("POST", "\/api\/teams", \{ name, config \}\),/,
    );
  });

  test("teamLoad POSTs /api/teams/{name}/load", () => {
    expect(client).toMatch(
      /teamLoad: \(name: string\) =>[\s\S]{1,400}`\/api\/teams\/\$\{encodeURIComponent\(name\)\}\/load`/,
    );
  });

  test("teamDuplicate POSTs /api/teams/{name}/duplicate with { new_name }", () => {
    expect(client).toMatch(
      /teamDuplicate: \(sourceName: string, newName: string\) =>[\s\S]{1,800}`\/api\/teams\/\$\{encodeURIComponent\(sourceName\)\}\/duplicate`,[\s\S]{1,400}\{ new_name: newName \}/,
    );
  });

  test("teamListLoaded GETs /api/teams/loaded", () => {
    expect(client).toMatch(
      /teamListLoaded: \(\) =>[\s\S]{1,200}req<\{ teams: string\[\] \}>\("GET", "\/api\/teams\/loaded"\),/,
    );
  });

  test("TeamConfigWire mirrors chan-drive's snake_case shape", () => {
    expect(client).toMatch(
      /export interface TeamConfigWire \{[\s\S]{1,400}team_name: string;[\s\S]{1,80}host_name: string;[\s\S]{1,80}host_handle: string;[\s\S]{1,80}auto_prefix_at: boolean;[\s\S]{1,80}created_at: string;[\s\S]{1,200}members: TeamMemberWire\[\];/,
    );
  });
});
