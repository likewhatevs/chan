import { describe, expect, test } from "vitest";
import tabs from "./tabs.svelte.ts?raw";
import pane from "../components/Pane.svelte?raw";

// `fullstack-a-68 slice 2`: Hybrid Nav transactional staging.
// T / O / P / G / N stage additions into the draft layout
// (and `stagedDraftEditors` queue for N); Enter materializes,
// Esc discards. Tests pin the state machine + helper shape.

describe("fullstack-a-68 slice 2: paneMode state extended with stagedDraftEditors", () => {
  test("paneMode singleton carries stagedDraftEditors as an array field", () => {
    expect(tabs).toMatch(
      /stagedDraftEditors: \{ paneId: string \}\[\];[\s\S]{1,400}stagedDraftEditors: \[\],/,
    );
  });

  test("enterPaneMode resets stagedDraftEditors to []", () => {
    expect(tabs).toMatch(
      /export function enterPaneMode\(\): void \{[\s\S]{1,800}paneMode\.stagedDraftEditors = \[\];/,
    );
  });

  test("commitPaneMode clears stagedDraftEditors as part of teardown", () => {
    expect(tabs).toMatch(
      /export function commitPaneMode\(\): void \{[\s\S]{1,1200}paneMode\.stagedDraftEditors = \[\];\s*\n\s*\}/,
    );
  });

  test("cancelPaneMode clears stagedDraftEditors as part of teardown", () => {
    expect(tabs).toMatch(
      /export function cancelPaneMode\(\): void \{[\s\S]{1,400}paneMode\.stagedDraftEditors = \[\];\s*\n\s*\}/,
    );
  });
});

describe("fullstack-a-68 slice 2: spawn helpers", () => {
  test("paneModeOpenRichPromptTerminal pushes a TerminalTab with richPrompt armed", () => {
    expect(tabs).toMatch(
      /export function paneModeOpenRichPromptTerminal\(ctx\?: SpawnContext\): void \{[\s\S]{1,2000}richPrompt: \{[\s\S]{1,400}open: true,/,
    );
  });

  test("paneModeStageDraftEditor pushes a {paneId} entry pinned at press time", () => {
    expect(tabs).toMatch(
      /export function paneModeStageDraftEditor\(\): void \{[\s\S]{1,400}const paneId = paneMode\.draft\.activePaneId;[\s\S]{1,200}paneMode\.stagedDraftEditors\.push\(\{ paneId \}\);/,
    );
  });

  test("paneModeStagedTabIds derives the staged set by diffing draft against live", () => {
    expect(tabs).toMatch(
      /export function paneModeStagedTabIds\(\): Set<string> \{[\s\S]{1,400}if \(!paneMode\.active \|\| !paneMode\.draft\) return new Set\(\);[\s\S]{1,2000}return staged;/,
    );
  });
});

describe("fullstack-a-68 slice 2: ghost-tab rendering in Pane.svelte", () => {
  test("Pane imports paneModeStagedTabIds + derives a Set", () => {
    expect(pane).toMatch(/paneModeStagedTabIds,/);
    expect(pane).toMatch(
      /const paneModeStagedSet = \$derived\(paneModeStagedTabIds\(\)\);/,
    );
  });

  test("tab DOM carries class:staged bound to the derived set", () => {
    expect(pane).toMatch(/class:staged=\{paneModeStagedSet\.has\(t\.id\)\}/);
  });

  test("CSS defines a dimmed dashed-border .tab.staged style", () => {
    expect(pane).toMatch(
      /\.tab\.staged \{[\s\S]{1,400}opacity: 0\.65;[\s\S]{1,200}border: 1px dashed/,
    );
  });
});
