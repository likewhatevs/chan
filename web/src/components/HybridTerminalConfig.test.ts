import { describe, expect, test } from "vitest";
import source from "./HybridTerminalConfig.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";

// Terminal preferences: scrollback range/clamp, TERM dropdown with a
// Custom sentinel, and device-wide warning copy.

describe("HybridTerminalConfig wiring", () => {
  test("warning copy distinguishes device-wide settings from body theme scope", () => {
    expect(source).toMatch(
      /Scrollback, TERM, and font apply to ALL terminals on this\s+device/,
    );
    expect(source).toMatch(/top-bar theme switch applies to ALL terminal\s+bodies/);
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
    // Re-fetching before PATCH is the safety net for a parallel
    // autosave from another back-of-card surface. The PATCH body
    // overlays only the terminal subtree.
    expect(source).toMatch(/const current = await api\.config\(\)/);
    expect(source).toMatch(
      /preferences: \{ \.\.\.current\.preferences, terminal: editing\.terminal \}/,
    );
    expect(source).toMatch(/await api\.updateConfig\(cfgBody\)/);
  });

  test("normalizeTerminal backfills missing fields with the defaults", () => {
    // Older servers may omit scrollback_mb or default_term; the
    // normalization keeps the form dirty()-stable after the post-save
    // re-clone.
    expect(source).toMatch(
      /p\.terminal\.scrollback_mb = clampScrollbackMb\(p\.terminal\.scrollback_mb\)/,
    );
    expect(source).toMatch(/p\.terminal\.default_term = term\.length > 0 \? term : DEFAULT_TERM/);
  });

  test("dirty check is scoped to the terminal subtree", () => {
    // Comparing the whole Preferences object would trigger spurious
    // PATCHes and clobber edits owned by other back-of-card surfaces.
    expect(source).toMatch(/function terminalDirty\(\): boolean/);
    expect(source).toMatch(
      /JSON\.stringify\(editing\.terminal \?\? null\)[\s\S]*?JSON\.stringify\(server \?\? null\)/,
    );
  });
});

describe("HybridTerminalConfig surface theme + custom-TERM fix", () => {
  test("top-bar body theme is delegated to the shared surface shell", () => {
    expect(source).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,160}title="Hybrid Terminal"[\s\S]{1,120}surface="terminal"/,
    );
    expect(source).not.toMatch(/<h3>Appearance<\/h3>/);
    expect(source).not.toMatch(/name="hybrid-terminal-theme-override"/);
  });

  test("onDone prop is accepted via $props", () => {
    expect(source).toMatch(
      /let \{ onDone \}: \{ onDone\?: \(\) => void \} = \$props\(\)/,
    );
  });

  test("customMode state tracks dropdown choice independently from default_term", () => {
    // Setting default_term="" after picking "Custom..." collapsed back
    // to DEFAULT_TERM via the currentTerm fallback, hiding the custom
    // input. Tracking "user chose Custom" in a separate state slot
    // prevents that collapse.
    expect(source).toMatch(/let customMode = \$state\(false\)/);
    expect(source).toMatch(
      /termSelectValue = \$derived\([\s\S]*?customMode\s*\?\s*CUSTOM_TERM_SENTINEL/,
    );
  });

  test("setTermSelection routes Custom selection through customMode", () => {
    // Choosing Custom must flip customMode = true and leave the
    // persisted default_term unchanged; seeding it to "" would trigger
    // the collapse bug.
    expect(source).toMatch(
      /if \(next === CUSTOM_TERM_SENTINEL\)\s*\{\s*customMode = true/,
    );
    expect(source).toMatch(
      /customMode = false;\s*editing\.terminal = \{ \.\.\.editing\.terminal, default_term: next \}/,
    );
  });

  test("custom-TERM input renders when termSelectValue is the sentinel", () => {
    // The {#if termSelectValue === CUSTOM_TERM_SENTINEL} block is the
    // gate. The fix lives in setTermSelection + customMode, not here.
    expect(source).toMatch(
      /\{#if termSelectValue === CUSTOM_TERM_SENTINEL\}[\s\S]*?class="custom-term"/,
    );
  });
});

describe("terminal-font dropdown + download flow", () => {
  test("dropdown carries both choices with the OS default as the leading option", () => {
    expect(source).toMatch(
      /<select[\s\S]*?id="hybrid-terminal-font"[\s\S]*?<option value="os-default">OS default \(mono\)<\/option>[\s\S]*?<option value="source-code-pro">Source Code Pro<\/option>/,
    );
  });

  test("setFontChoice fires the download endpoint when user opts into SCP", () => {
    // Choosing Source Code Pro triggers the POST that fetches the woff2
    // into <user-config>/chan/fonts/.
    expect(source).toMatch(/api\.fontsSourceCodeProDownload\(\)/);
  });

  test("download failure rolls the preference back to os-default", () => {
    // Failure must roll back so the next session mounts the OS-default
    // font chain rather than claiming SCP when its file is missing.
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

describe("MCP env-var toggle", () => {
  test("checkbox reads + writes the persisted mcp_env field", () => {
    expect(source).toMatch(
      /id="hybrid-terminal-mcp-env"[\s\S]{1,160}type="checkbox"[\s\S]{1,160}checked=\{mcpEnvOn\}/,
    );
    expect(source).toMatch(/const mcpEnvOn = \$derived\(editing\?\.terminal\?\.mcp_env === true\)/);
    expect(source).toMatch(
      /editing\.terminal = \{ \.\.\.editing\.terminal, mcp_env: next \}/,
    );
  });

  test("hint names the CHAN_MCP_* discovery vars + spawn-time scope", () => {
    expect(source).toContain("CHAN_MCP_SOCKET");
    expect(source).toContain("CHAN_MCP_SERVER_JSON");
    expect(source).toMatch(/applies to newly spawned terminals/);
  });
});

describe("Wave 4: Terminal back-side controls", () => {
  test("OK button routes through onDone", () => {
    expect(source).toMatch(/<HybridSurfaceConfigShell[\s\S]*?\{onDone\}/);
    expect(shell).toMatch(
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

describe("post-save effect_update_depth_exceeded guard", () => {
  // The hydration $effect must not reassign `editing` to a
  // content-identical clone on every workspace.info change.
  // That replaces the $state proxy and re-fires the effect ->
  // Svelte 5 trips effect_update_depth_exceeded (UI freeze).
  // The fix tracks the JSON of the server's `preferences.terminal`
  // slice and bails when the slice is unchanged.
  test("tracks lastSyncedServerSnap across workspace.info refreshes", () => {
    expect(source).toMatch(
      /let lastSyncedServerSnap: string \| null = null;/,
    );
  });

  test("hydration effect bails when the server terminal slice hasn't changed", () => {
    expect(source).toMatch(
      /\$effect\(\(\) => \{[\s\S]{1,2000}const serverSnap = JSON\.stringify\(info\.preferences\?\.terminal \?\? null\);[\s\S]{1,400}if \(editing && serverSnap === lastSyncedServerSnap\) \{[\s\S]{1,800}return;[\s\S]{1,200}\}[\s\S]{1,200}lastSyncedServerSnap = serverSnap;[\s\S]{1,200}editing = normalizeTerminal\(clone\(info\.preferences\)\);/,
    );
  });
});
