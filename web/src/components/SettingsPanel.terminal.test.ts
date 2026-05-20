import { describe, expect, test } from "vitest";
import panel from "./SettingsPanel.svelte?raw";

// `fullstack-b-11`: the Settings page exposes two new terminal
// preferences (per-terminal scrollback in MB and the default TERM
// env var). These pinned-source assertions guard the wiring so a
// future refactor can't silently drop the section or swap the
// settings path away from the persisted `Preferences.terminal`
// subtree the chan-server-side `sanitize_terminal_config` writes
// against.

describe("fullstack-b-11: SettingsPanel terminal section", () => {
  test("section header and spawn-time-only hint copy are present", () => {
    expect(panel).toMatch(/<h3>Terminal<\/h3>/);
    expect(panel).toMatch(/Applies to terminals spawned after this setting/);
  });

  test("scrollback control reads + writes the persisted MB field", () => {
    // Range input.
    expect(panel).toMatch(
      /id="terminal-scrollback-mb"[\s\S]*?type="range"[\s\S]*?value=\{scrollbackMb\}/,
    );
    // Mirror number input for keyboard entry.
    expect(panel).toMatch(/class="scrollback-number"[\s\S]*?type="number"/);
    // Both controls dispatch through setScrollbackMb -> clamp helper.
    const setterMatches = panel.match(/setScrollbackMb\(/g) ?? [];
    expect(setterMatches.length).toBeGreaterThanOrEqual(3);
  });

  test("scrollback range uses the shared MB bounds", () => {
    expect(panel).toMatch(/min=\{SCROLLBACK_MB_MIN\}/);
    expect(panel).toMatch(/max=\{SCROLLBACK_MB_MAX\}/);
  });

  test("TERM dropdown ships the four known terminfo entries + Custom", () => {
    expect(panel).toMatch(/KNOWN_TERM_VALUES = \[/);
    expect(panel).toContain('"xterm-256color"');
    expect(panel).toContain('"xterm"');
    expect(panel).toContain('"tmux-256color"');
    expect(panel).toContain('"screen-256color"');
    expect(panel).toMatch(/value=\{CUSTOM_TERM_SENTINEL\}>Custom\.\.\.<\/option>/);
  });

  test("custom TERM input renders only when the sentinel is active", () => {
    expect(panel).toMatch(
      /\{#if termSelectValue === CUSTOM_TERM_SENTINEL\}[\s\S]*?class="custom-term"/,
    );
  });

  test("normalizePrefs backfills missing terminal fields with the defaults", () => {
    // Pre-fullstack-b-11 servers don't ship scrollback_mb /
    // default_term; normalizePrefs is the single seam that keeps the
    // form dirty()-stable after the post-save re-clone.
    expect(panel).toMatch(
      /p\.terminal\.scrollback_mb = clampScrollbackMb\(p\.terminal\.scrollback_mb\)/,
    );
    expect(panel).toMatch(/p\.terminal\.default_term = term\.length > 0 \? term : DEFAULT_TERM/);
  });

  test("section is wired into the active template, not just declared", () => {
    expect(panel).toMatch(/class="terminal-section"/);
    expect(panel).toMatch(/class="terminal-field"/);
  });
});
