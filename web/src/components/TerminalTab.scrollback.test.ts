import { describe, expect, test } from "vitest";
import tab from "./TerminalTab.svelte?raw";

// `fullstack-b-11`: TerminalTab must read its xterm.js scrollback
// cap from the persisted MB setting at construction time, not the
// pre-fix 20_000-line constant. A future regression that re-hardcodes
// the cap (or accidentally drops the spawn-time read) shows up here.

describe("fullstack-b-11: TerminalTab scrollback wiring", () => {
  test("scrollback line cap is held on the component, not inline-literal in the xterm config", () => {
    expect(tab).toMatch(/let scrollbackLines = scrollbackLinesFromMb\(/);
    expect(tab).toMatch(/scrollback: scrollbackLines,/);
    // The bare-20000 literal in `new Terminal({ scrollback: 20_000 })`
    // is exactly what this task removes.
    expect(tab).not.toMatch(/scrollback: 20_000/);
  });

  test("start() recomputes the line cap from current Preferences", () => {
    expect(tab).toMatch(
      /scrollbackLines = scrollbackLinesFromMb\(\s*clampScrollbackMb\(drive\.info\?\.preferences\?\.terminal\?\.scrollback_mb\),?\s*\)/,
    );
  });

  test("copy-scrollback actions use the configured cap, not a hardcoded constant", () => {
    // Both serialize call-sites must thread the per-component value
    // so the "copy scrollback" menu matches the buffer actually held.
    const matches = tab.match(/serialize\?\.serialize\(\{ scrollback: scrollbackLines \}\)/g) ?? [];
    expect(matches.length).toBeGreaterThanOrEqual(2);
  });
});
