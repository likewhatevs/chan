import { describe, expect, test } from "vitest";
import orchestrator from "./teamOrchestrator.svelte.ts?raw";
import tabs from "./tabs.svelte.ts?raw";
import { resolveMemberPaneIds } from "./teamOrchestrator.svelte";
import type {
  TeamDialogConfig,
  TeamMemberDraft,
} from "./teamDialog.svelte";

// `fullstack-a-79` slice 4: split-pane real estate. Tests pin
// the architectural shape of the `buildSplitGrid` helper +
// the orchestrator's per-member pane-id resolution +
// the wiring in the bootstrap chain.

describe("fullstack-a-79 slice 4: buildSplitGrid helper", () => {
  test("declared in tabs.svelte with the documented row-major contract", () => {
    expect(tabs).toMatch(
      /export function buildSplitGrid\(\s*startPaneId: string,\s*rows: number,\s*cols: number,?\s*\): string\[\] \{/,
    );
  });

  test("1×1 short-circuits to the starting pane (no splits)", () => {
    expect(tabs).toMatch(
      /if \(rows <= 1 && cols <= 1\) return \[startPaneId\];/,
    );
  });

  test("step 1: builds top-row column heads via direction:\"row\" splits", () => {
    expect(tabs).toMatch(
      /const columnHeads: string\[\] = \[startPaneId\];[\s\S]{1,800}splitPane\(columnHeads\[c - 1\], "row", "after"\);[\s\S]{1,400}columnHeads\.push\(layout\.activePaneId\);/,
    );
  });

  test("step 2: for each column-head, splits direction:\"column\" to populate rows", () => {
    expect(tabs).toMatch(
      /splitPane\(grid\[r - 1\]\[c\], "column", "after"\);[\s\S]{1,400}grid\[r\]\[c\] = layout\.activePaneId;/,
    );
  });

  test("returns row-major flat array", () => {
    expect(tabs).toMatch(
      /for \(let r = 0; r < rows; r \+= 1\) \{[\s\S]{1,400}for \(let c = 0; c < cols; c \+= 1\) flat\.push\(grid\[r\]\[c\]\);/,
    );
  });
});

describe("fullstack-a-79 slice 4: resolveMemberPaneIds behaviour", () => {
  function makeConfig(
    realEstate: TeamDialogConfig["realEstate"],
    memberCount = 3,
  ): TeamDialogConfig {
    const members: TeamMemberDraft[] = [];
    for (let i = 0; i < memberCount; i += 1) {
      members.push({
        name: i === 0 ? "Lead" : `Worker${i}`,
        command: "claude",
        env: "",
        isLead: i === 0,
      });
    }
    return {
      hostName: "Alice",
      teamName: "demo",
      size: memberCount,
      autoPrefix: true,
      members,
      realEstate,
    };
  }

  test("tabs mode: every member resolves to layout.activePaneId", () => {
    const cfg = makeConfig({ kind: "tabs" });
    const { lead, workers } = resolveMemberPaneIds(cfg);
    expect(typeof lead).toBe("string");
    expect(workers.length).toBe(3);
    expect(workers.every((p) => p === lead)).toBe(true);
  });

  test("split mode 1×1: degenerate (no splits), all members on the starting pane", () => {
    const cfg = makeConfig({
      kind: "split",
      grid: { rows: 1, cols: 1 },
      slots: [[0, 1, 2]],
    });
    const { lead, workers } = resolveMemberPaneIds(cfg);
    expect(workers.every((p) => p === lead)).toBe(true);
  });
});

describe("fullstack-a-79 slice 4: orchestrator wiring", () => {
  test("imports buildSplitGrid + setActivePane + openTerminalInPane + layout", () => {
    expect(orchestrator).toMatch(
      /import \{[\s\S]{1,800}buildSplitGrid,[\s\S]{1,400}layout,[\s\S]{1,400}openTerminalInPane,[\s\S]{1,400}setActivePane,[\s\S]{1,200}\} from "\.\/tabs\.svelte";/,
    );
  });

  test("resolveMemberPaneIds is exported", () => {
    expect(orchestrator).toMatch(
      /export function resolveMemberPaneIds\(\s*config: TeamDialogConfig,?\s*\): \{ lead: string; workers: \(string \| undefined\)\[\] \} \{/,
    );
  });

  test("split branch reads grid + slots from realEstate", () => {
    expect(orchestrator).toMatch(
      /const \{ grid, slots \} = config\.realEstate;[\s\S]{1,400}const cells = buildSplitGrid\(startId, grid\.rows, grid\.cols\);/,
    );
  });

  test("slots are inverted to per-member pane assignments (slots[cellIdx] is a list of memberIdx)", () => {
    expect(orchestrator).toMatch(
      /for \(let cellIdx = 0; cellIdx < slots\.length; cellIdx \+= 1\) \{[\s\S]{1,800}for \(const memberIdx of memberIdxs\) \{[\s\S]{1,800}workers\[memberIdx\] = cells\[cellIdx\] \?\? fallback;/,
    );
  });

  test("orchestrator calls openTerminalInPane(paneId, …) for each worker (not openTerminalInActivePane)", () => {
    expect(orchestrator).toMatch(
      /openTerminalInPane\(paneId, \{[\s\S]{1,400}sessionId: response\.session,/,
    );
    expect(orchestrator).not.toMatch(/openTerminalInActivePane\(\{/);
  });

  test("setActivePane(leadPaneId) restores focus after the spawn loop", () => {
    expect(orchestrator).toMatch(
      /if \(leadPaneId\) setActivePane\(leadPaneId\);/,
    );
  });

  test("slice-1 split-pane scope-poke notify is GONE", () => {
    expect(orchestrator).not.toMatch(
      /Split-pane real estate not yet wired/,
    );
  });
});
