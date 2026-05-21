import { describe, expect, test } from "vitest";
import panel from "./SettingsPanel.svelte?raw";

// `fullstack-a-45` (Task B): the Terminal section originally added
// by `fullstack-b-11` moved out of SettingsPanel into
// HybridTerminalConfig.svelte. The wiring assertions migrated with
// it (see HybridTerminalConfig.test.ts). This file shrinks to a
// regression guard: re-adding Terminal markup or its supporting
// TERM constants to SettingsPanel would re-introduce the duplicate
// surface the `-a-45` migration removed.

describe("fullstack-a-45: Terminal section removed from SettingsPanel", () => {
  test("Terminal section header is not present in SettingsPanel", () => {
    expect(panel).not.toMatch(/<h3>Terminal<\/h3>/);
    expect(panel).not.toMatch(/class="terminal-section"/);
  });

  test("Terminal control ids are not present in SettingsPanel", () => {
    expect(panel).not.toMatch(/id="terminal-scrollback-mb"/);
    expect(panel).not.toMatch(/id="terminal-default-term"/);
  });

  test("TERM constants no longer declared in SettingsPanel", () => {
    expect(panel).not.toMatch(/KNOWN_TERM_VALUES = \[/);
    expect(panel).not.toMatch(/CUSTOM_TERM_SENTINEL/);
  });

  test("scrollback helpers no longer imported into SettingsPanel", () => {
    expect(panel).not.toMatch(/from "\.\.\/terminal\/scrollback"/);
  });

  test("normalizePrefs no longer touches the terminal subtree", () => {
    // The Hybrid Terminal config component owns terminal field
    // normalization now. If SettingsPanel starts re-normalizing
    // the subtree it implies the migration regressed.
    expect(panel).not.toMatch(/p\.terminal\.scrollback_mb/);
    expect(panel).not.toMatch(/p\.terminal\.default_term/);
  });
});
