import { describe, expect, test } from "vitest";
import terminal from "./HybridTerminalConfig.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";

// The Source Code Pro on/off control lives in HybridTerminalConfig
// (the terminal back-of-card), next to the other terminal-scoped
// settings. The Dashboard About slide keeps the attribution + OFL link
// but not the toggle (terminal font is a terminal concern).

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

  test("Dashboard About slide keeps the SCP attribution + OFL link but no toggle", () => {
    expect(carousel).toMatch(/Source Code Pro Regular/);
    // The OFL link points at the canonical upstream URL rather than
    // the embedded /static/fonts/OFL.txt path (which resolves to
    // 127.0.0.1 under the desktop non-root mount).
    expect(carousel).toMatch(
      /href="https:\/\/github\.com\/adobe-fonts\/source-code-pro\/blob\/release\/LICENSE\.md"/,
    );
    expect(carousel).not.toMatch(/setFontChoice\(/);
    expect(carousel).not.toMatch(/value="source-code-pro"/);
    expect(carousel).not.toMatch(/fontsSourceCodeProDownload/);
  });
});
