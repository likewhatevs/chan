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
    // `-a-53` updated the warning copy when the Appearance
    // override toggle joined this surface — Scrollback + TERM
    // are device-wide; Appearance is per-Hybrid.
    expect(source).toMatch(
      /Scrollback and TERM apply to ALL terminals on this device/,
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

describe("fullstack-a-53: HybridTerminalConfig per-Hybrid override + custom-TERM fix", () => {
  test("per-Hybrid Appearance override radios bind pane.theme", () => {
    expect(source).toMatch(/name="hybrid-terminal-theme-override"/);
    expect(source).toContain('"inherit"');
    expect(source).toContain('"light"');
    expect(source).toContain('"dark"');
    expect(source).toMatch(/setOverrideChoice\(opt\.value\)/);
    expect(source).toMatch(/pane\.theme = next/);
    expect(source).toMatch(/pane\.theme = undefined/);
  });

  test("pane and onDone props are accepted via $props", () => {
    expect(source).toMatch(
      /let \{ pane, onDone \}: \{ pane: LeafNode; onDone\?: \(\) => void \} = \$props\(\)/,
    );
  });

  test("custom-TERM fix: customMode state tracks dropdown choice (-a-53)", () => {
    // The `-a-45` PARTIAL: setting default_term="" after picking
    // "Custom..." collapsed back to DEFAULT_TERM via the
    // currentTerm fallback, snapping termSelectValue to a known
    // entry and hiding the custom input. The fix tracks
    // "user chose Custom" in a separate state slot.
    expect(source).toMatch(/let customMode = \$state\(false\)/);
    expect(source).toMatch(
      /termSelectValue = \$derived\([\s\S]*?customMode\s*\?\s*CUSTOM_TERM_SENTINEL/,
    );
  });

  test("setTermSelection routes Custom selection through customMode (-a-53)", () => {
    // setTermSelection("__custom__") must flip customMode = true
    // and leave the persisted default_term ALONE (don't seed it
    // to "" the way the old code did — that's what triggered the
    // PARTIAL).
    expect(source).toMatch(
      /if \(next === CUSTOM_TERM_SENTINEL\)\s*\{\s*customMode = true/,
    );
    expect(source).toMatch(
      /customMode = false;\s*editing\.terminal = \{ \.\.\.editing\.terminal, default_term: next \}/,
    );
  });

  test("custom-TERM input renders when termSelectValue is the sentinel", () => {
    // Pinning the markup wiring — the {#if termSelectValue ===
    // CUSTOM_TERM_SENTINEL} block stays the conditional gate on
    // the custom input. Fix shape is in setTermSelection +
    // customMode, NOT in the markup.
    expect(source).toMatch(
      /\{#if termSelectValue === CUSTOM_TERM_SENTINEL\}[\s\S]*?class="custom-term"/,
    );
  });
});

describe("fullstack-b-30 slice b: terminal-font dropdown + download flow", () => {
  test("dropdown carries both choices with the OS default as the leading option", () => {
    expect(source).toMatch(
      /<select[\s\S]*?id="hybrid-terminal-font"[\s\S]*?<option value="os-default">OS default \(mono\)<\/option>[\s\S]*?<option value="source-code-pro">Source Code Pro<\/option>/,
    );
  });

  test("setFontChoice fires the download endpoint when user opts into SCP", () => {
    // Slice b's user-facing piece: choosing Source Code Pro
    // triggers the POST endpoint that fetches the woff2 + OFL
    // into <user-config>/chan/fonts/.
    expect(source).toMatch(/api\.fontsSourceCodeProDownload\(\)/);
  });

  test("download failure rolls the preference back to os-default", () => {
    // The SPA never claims SCP is active while the user-config
    // file is missing — failure rolls back so the next reload
    // mounts the OS-default chain.
    expect(source).toMatch(
      /editing\.terminal = \{[^}]*?font: "os-default"[^}]*?\};/,
    );
    expect(source).toMatch(/Source Code Pro download failed/);
  });

  test("dropdown is disabled while the download is in flight", () => {
    expect(source).toMatch(/disabled=\{fontDownloading\}/);
  });

  test("hint copy names the per-OS native faces + the download size + spawn-time-only contract", () => {
    expect(source).toMatch(/SF Mono on[\s\S]*?macOS/);
    expect(source).toMatch(/Cascadia on[\s\S]*?Windows/);
    expect(source).toMatch(/DejaVu on[\s\S]*?Linux/);
    expect(source).toMatch(/Spawn-time-only/);
  });
});

describe("Wave 4: Terminal back-side controls", () => {
  test("OK button routes through onDone", () => {
    expect(source).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("TERM and font dropdowns use the polished config-select style", () => {
    expect(source).toMatch(
      /id="hybrid-terminal-default-term"[\s\S]{1,120}class="config-select family"/,
    );
    expect(source).toMatch(
      /id="hybrid-terminal-font"[\s\S]{1,120}class="config-select family"/,
    );
    expect(source).toMatch(/\.config-select \{[\s\S]{1,300}border: 1px solid var\(--border\)/);
  });
});
