import { describe, expect, test } from "vitest";
import terminal from "./HybridTerminalConfig.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";

// Phase-13 round-1 closing (B10): the Source Code Pro on/off
// control is owned by HybridTerminalConfig.svelte (the
// terminal back-of-card) so it lives next to the other
// terminal-scoped settings the dropdown reads as. The
// Dashboard About slide keeps the attribution copy + the OFL
// license link, but it does NOT carry the toggle — that would
// have lived in the wrong surface (terminal font is a terminal
// concern, not a workspace-meta concern).

describe("phase-13 round-1 closing B10: Source Code Pro toggle placement", () => {
  test("Terminal back-of-card carries the Source Code Pro option + setFontChoice wiring", () => {
    expect(terminal).toMatch(/<option value="source-code-pro">Source Code Pro<\/option>/);
    expect(terminal).toMatch(/<option value="os-default">OS default \(mono\)<\/option>/);
    expect(terminal).toMatch(/async function setFontChoice\(/);
    expect(terminal).toMatch(/await api\.fontsSourceCodeProDownload\(\)/);
    // OS default rollback path so "off" really does fall back to
    // the system monospace font when the download fails.
    expect(terminal).toMatch(/editing\.terminal = \{ \.\.\.editing\.terminal, font: "os-default" \}/);
  });

  test("Dashboard About slide keeps the SCP attribution + OFL link but no toggle", () => {
    expect(carousel).toMatch(/Source Code Pro Regular/);
    expect(carousel).toMatch(/href="\/static\/fonts\/OFL\.txt"/);
    // No toggle wiring lives on the carousel side — terminal
    // font is a terminal-back-of-card concern.
    expect(carousel).not.toMatch(/setFontChoice\(/);
    expect(carousel).not.toMatch(/value="source-code-pro"/);
    expect(carousel).not.toMatch(/fontsSourceCodeProDownload/);
  });
});
