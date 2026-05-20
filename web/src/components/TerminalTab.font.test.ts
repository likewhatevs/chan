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

describe("fullstack-b-12: TerminalTab font + cursor parity", () => {
  test("xterm.js fontFamily lists Source Code Pro before fallbacks", () => {
    expect(tab).toMatch(
      /fontFamily:\s*'"Source Code Pro",[^']*?"SF Mono"[^']*?Menlo[^']*?Consolas/,
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
