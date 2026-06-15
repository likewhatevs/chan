import { describe, expect, test } from "vitest";
import tab from "./TerminalTab.svelte?raw";

// Phase-26: friendly in-app gate when Git for Windows is missing. The
// backend contract (chan-server terminal_sessions.rs / routes/terminal.rs,
// pinned by a Rust test) is a WS error frame `reason: "git_bash_missing"`
// and an HTTP 424 on the restart path. TerminalTab consumes BOTH and
// renders the gate instead of a raw error. (Component is Svelte, so the
// wiring is asserted as source shape; the render is browser-smoked.)

describe("missing-Git in-app gate", () => {
  test("consumes the WS git_bash_missing error frame", () => {
    // The reason tag matches the pinned backend constant.
    expect(tab).toMatch(/GIT_BASH_MISSING_REASON = "git_bash_missing"/);
    // The error-frame handler flips the gate flag on that reason.
    expect(tab).toMatch(
      /frame\.reason === GIT_BASH_MISSING_REASON\)\s*\{\s*gitBashMissing = true/,
    );
  });

  test("consumes HTTP 424 on the restart path", () => {
    expect(tab).toMatch(
      /err instanceof ApiError && err\.status === 424\)\s*\{\s*gitBashMissing = true/,
    );
  });

  test("renders a friendly gate with an Install link, not a raw error", () => {
    expect(tab).toMatch(/\{#if gitBashMissing\}/);
    expect(tab).toMatch(/Git for Windows is required for the terminal/);
    expect(tab).toMatch(
      /GIT_FOR_WINDOWS_URL = "https:\/\/gitforwindows\.org\/"/,
    );
    expect(tab).toMatch(/openExternalUrl\(GIT_FOR_WINDOWS_URL\)/);
  });

  test("the gate flag clears on the next (re)connect attempt", () => {
    // connect() resets the per-attempt status block, including the gate.
    expect(tab).toMatch(/sawSessionControl = false;\s*gitBashMissing = false;/);
  });
});
