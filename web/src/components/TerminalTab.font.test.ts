import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";
import tab from "./TerminalTab.svelte?raw";
import main from "../main.ts?raw";

// `?raw` returns an empty string for `.css` imports under the JSDOM
// vitest setup (the CSS plugin chain consumes them); read the file
// from disk relative to the vitest cwd (= web/) instead.
const fonts = readFileSync("src/fonts.css", "utf8");

// `fullstack-b-12`: chan ships Source Code Pro Regular for the
// in-app terminal and pins xterm.js to non-blinking block cursor +
// 14 pt to match iTerm2's defaults. The pinned-source assertions
// guard the wiring so a future refactor can't silently drop the
// bundled face or revert the cursor defaults.
//
// `fullstack-b-30` slice a: per-OS native mono now leads the
// fontFamily chain; Source Code Pro stays in the chain but only
// kicks in when the user opts in via Settings (slice b
// follow-up). The "SCP before fallbacks" pin from `-b-12` is
// inverted accordingly — SCP must now appear AFTER the OS-native
// faces. The font + OFL pins below stay (the face still ships
// when `embed-font` is on; the @font-face declaration still
// renders the loadable face URL).

describe("TerminalTab font + cursor parity", () => {
  test("OS-default font chain leads with per-OS native mono and trails with Source Code Pro", () => {
    // `fullstack-b-30` slice b: literal fontFamily inlined string
    // moved into a named constant (`FONT_CHAIN_OS_DEFAULT`) so the
    // pref-driven swap can pick between two chains. Pin the
    // constant body rather than the prior inline `fontFamily:`.
    expect(tab).toMatch(
      /FONT_CHAIN_OS_DEFAULT\s*=\s*'"SF Mono",[^']*?"Cascadia Code"[^']*?"DejaVu Sans Mono"[^']*?"Source Code Pro"/,
    );
  });

  test("Source Code Pro font chain leads with SCP when the user opts in", () => {
    expect(tab).toMatch(
      /FONT_CHAIN_SOURCE_CODE_PRO\s*=\s*'"Source Code Pro",[^']*?"SF Mono"/,
    );
  });

  test("fontFamily reads the persisted preference at spawn time", () => {
    // Spawn-time read of `workspace.info.preferences.terminal.font`
    // mirrors `-b-11`'s scrollback contract: existing terminals
    // keep their font until session restart.
    expect(tab).toMatch(
      /workspace\.info\?\.preferences\?\.terminal\?\.font\s*\?\?\s*"os-default"/,
    );
    expect(tab).toMatch(
      /fontPref === "source-code-pro"\s*\?\s*FONT_CHAIN_SOURCE_CODE_PRO\s*:\s*FONT_CHAIN_OS_DEFAULT/,
    );
  });

  test("fontSize matches iTerm reference (14)", () => {
    expect(tab).toMatch(/fontSize:\s*14,/);
  });

  test("cursor is non-blinking block per iTerm defaults", () => {
    expect(tab).toMatch(/cursorBlink:\s*false,/);
    expect(tab).toMatch(/cursorStyle:\s*"block",/);
  });

  test("@font-face declaration points at the rust-embed-served path", () => {
    expect(fonts).toMatch(/font-family:\s*['"]Source Code Pro['"]/);
    expect(fonts).toMatch(/font-weight:\s*400/);
    expect(fonts).toMatch(
      /url\(['"]?\/static\/fonts\/SourceCodePro-Regular\.otf\.woff2['"]?\)/,
    );
    expect(fonts).toMatch(/font-display:\s*swap/);
  });

  test("fonts.css is imported at app boot so the face starts loading early", () => {
    expect(main).toMatch(/import\s+"\.\/fonts\.css"/);
  });
});
