import { describe, expect, test } from "vitest";
import helpSource from "./PaneModeHelp.svelte?raw";

// Standalone terminal windows (`?kind=terminal`) have no workspace: no
// file browser, graph, drafts, team work, or search. App.svelte already
// guards the matching handlePaneModeKey cases on `ui.terminalOnly`; the
// Hybrid Nav cheatsheet must hide the same commands or it advertises
// (clickable!) actions that silently no-op. These pins lock the overlay
// filter to the handler gating.
describe("PaneModeHelp terminal-only filtering", () => {
  const src = helpSource.replace(/\s+/g, " ");

  test("reads terminal-only mode from the store", () => {
    expect(src).toContain('import { ui } from "../state/store.svelte"');
  });

  test("every workspace-surface stage row is tagged workspaceOnly", () => {
    // One pin per row, anchored on the action string, so a reorder or
    // comment churn can't break them and a dropped tag fails loudly.
    for (const action of [
      "Stage File Browser",
      "Stage Graph",
      "Stage New Draft",
      "Search overlay",
    ]) {
      expect(src).toMatch(
        new RegExp(`action: "${action}", workspaceOnly: true`),
      );
    }
  });

  test("Stage Terminal stays available in terminal-only mode", () => {
    expect(src).toContain('action: "Stage Terminal" }');
    expect(src).not.toMatch(/action: "Stage Terminal", workspaceOnly/);
  });

  test("the Dock group (file-browser docks) is tagged as a whole", () => {
    expect(src).toMatch(/title: "Dock", workspaceOnly: true/);
  });

  test("groups is a pure $derived filter over ui.terminalOnly", () => {
    expect(src).toContain("const groups = $derived(");
    expect(src).toContain("ui.terminalOnly");
    expect(src).toContain("ALL_GROUPS.filter((g) => !g.workspaceOnly)");
    expect(src).toContain("g.rows.filter((r) => !r.workspaceOnly)");
    // Empty groups (Dock) drop out instead of rendering a bare header.
    expect(src).toContain(".filter((g) => g.rows.length > 0)");
  });
});
