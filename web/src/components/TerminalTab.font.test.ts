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

describe("fullstack-b-12 + fullstack-b-30: TerminalTab font + cursor parity", () => {
  test("xterm.js fontFamily leads with per-OS native mono and trails with Source Code Pro", () => {
    expect(tab).toMatch(
      /fontFamily:\s*'"SF Mono",[^']*?"Cascadia Code"[^']*?"DejaVu Sans Mono"[^']*?"Source Code Pro"/,
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
