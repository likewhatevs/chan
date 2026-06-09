import { describe, expect, test } from "vitest";
import terminal from "./HybridTerminalConfig.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";

// The Source Code Pro on/off control lives in HybridTerminalConfig
// (the terminal back-of-card), next to the other terminal-scoped
// settings. The Dashboard About slide carries neither the toggle nor the
// font attribution (the third-party attributions were dropped).

describe("Source Code Pro toggle placement", () => {
  test("Terminal back-of-card carries the Source Code Pro option + setFontChoice wiring", () => {
    expect(terminal).toMatch(/<option value="source-code-pro">Source Code Pro<\/option>/);
    expect(terminal).toMatch(/<option value="os-default">OS default \(mono\)<\/option>/);
    expect(terminal).toMatch(/async function setFontChoice\(/);
    expect(terminal).toMatch(/await api\.fontsSourceCodeProDownload\(\)/);
    // OS default rollback path so "off" really does fall back to
    // the system monospace font when the download fails.
    expect(terminal).toMatch(/editing\.terminal = \{ \.\.\.editing\.terminal, font: "os-default" \}/);
  });

  test("Dashboard About slide carries neither the SCP attribution nor the toggle", () => {
    // The terminal-font attribution + OFL link were dropped from the
    // About slide; the toggle always lived on the terminal back-of-card.
    expect(carousel).not.toMatch(/Source Code Pro Regular/);
    expect(carousel).not.toMatch(
      /href="https:\/\/github\.com\/adobe-fonts\/source-code-pro\/blob\/release\/LICENSE\.md"/,
    );
    expect(carousel).not.toMatch(/setFontChoice\(/);
    expect(carousel).not.toMatch(/value="source-code-pro"/);
    expect(carousel).not.toMatch(/fontsSourceCodeProDownload/);
  });
});
