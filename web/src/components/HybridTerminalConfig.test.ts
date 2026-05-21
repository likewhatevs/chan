import { describe, expect, test } from "vitest";
import source from "./HybridTerminalConfig.svelte?raw";

// `fullstack-a-45` (Task B): the Terminal preferences UI migrated
// out of SettingsPanel into HybridTerminalConfig. These pins
// carry over the wiring guarantees the original
// `SettingsPanel.terminal.test.ts` from `fullstack-b-11` enforced
// (scrollback range / number / clamp; TERM dropdown ships the
// known terminfo entries plus Custom sentinel; custom input
// renders only when the sentinel is active) and adds new pins
// for the warning copy required by `-a-45`.

describe("fullstack-a-45: HybridTerminalConfig wiring", () => {
  test("warning copy distinguishes device-wide scope from per-pane", () => {
    expect(source).toMatch(
      /These settings apply to ALL terminals, not just this one\./,
    );
    expect(source).toMatch(/class="hint warning"/);
  });

  test("scrollback control reads + writes the persisted MB field", () => {
    expect(source).toMatch(
      /id="hybrid-terminal-scrollback-mb"[\s\S]*?type="range"[\s\S]*?value=\{scrollbackMb\}/,
    );
    expect(source).toMatch(/class="scrollback-number"[\s\S]*?type="number"/);
    const setterMatches = source.match(/setScrollbackMb\(/g) ?? [];
    expect(setterMatches.length).toBeGreaterThanOrEqual(3);
  });

  test("scrollback range uses the shared MB bounds", () => {
    expect(source).toMatch(/min=\{SCROLLBACK_MB_MIN\}/);
    expect(source).toMatch(/max=\{SCROLLBACK_MB_MAX\}/);
    expect(source).toMatch(/from "\.\.\/terminal\/scrollback"/);
  });

  test("TERM dropdown ships the four known terminfo entries + Custom", () => {
    expect(source).toMatch(/KNOWN_TERM_VALUES = \[/);
    expect(source).toContain('"xterm-256color"');
    expect(source).toContain('"xterm"');
    expect(source).toContain('"tmux-256color"');
    expect(source).toContain('"screen-256color"');
    expect(source).toMatch(/value=\{CUSTOM_TERM_SENTINEL\}>Custom\.\.\.<\/option>/);
  });

  test("custom TERM input renders only when the sentinel is active", () => {
    expect(source).toMatch(
      /\{#if termSelectValue === CUSTOM_TERM_SENTINEL\}[\s\S]*?class="custom-term"/,
    );
  });

  test("save path re-fetches global config before PATCH to avoid clobbering parallel edits", () => {
    // The merge-with-current-server pattern is the safety net for
    // SettingsPanel running an autosave in parallel. The PATCH
    // body must overlay only the terminal subtree onto whatever
    // the server currently has.
    expect(source).toMatch(/const current = await api\.config\(\)/);
    expect(source).toMatch(
      /preferences: \{ \.\.\.current\.preferences, terminal: editing\.terminal \}/,
    );
    expect(source).toMatch(/await api\.updateConfig\(cfgBody\)/);
  });

  test("normalizeTerminal backfills missing fields with the defaults", () => {
    // Pre-fullstack-b-11 servers don't ship scrollback_mb /
    // default_term; the normalization is the single seam that
    // keeps the form dirty()-stable after the post-save re-clone.
    expect(source).toMatch(
      /p\.terminal\.scrollback_mb = clampScrollbackMb\(p\.terminal\.scrollback_mb\)/,
    );
    expect(source).toMatch(/p\.terminal\.default_term = term\.length > 0 \? term : DEFAULT_TERM/);
  });

  test("dirty check is scoped to the terminal subtree", () => {
    // The dirty comparator must NOT compare the whole Preferences
    // object — that would react to SettingsPanel-owned edits and
    // fire a spurious PATCH (worse: a PATCH from this surface
    // could clobber theme / editor / date edits SettingsPanel
    // hadn't yet flushed).
    expect(source).toMatch(/function terminalDirty\(\): boolean/);
    expect(source).toMatch(
      /JSON\.stringify\(editing\.terminal \?\? null\)[\s\S]*?JSON\.stringify\(server \?\? null\)/,
    );
  });
});
