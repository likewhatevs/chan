import { describe, expect, test } from "vitest";
import tab from "./TerminalTab.svelte?raw";

// `fullstack-b-29`: the xterm.js default DOM renderer renders
// box-drawing + block-element characters via the system font,
// which (under chan's `lineHeight: 1.2`) leaves vertical gaps
// between cell corners. The WebGL renderer fires xterm.js's
// built-in customGlyphs path that draws pixel-perfect glyphs
// filling the entire cell rectangle including the line-height
// padding. These pins guard the WebGL wiring so a future
// refactor can't silently revert to DOM + reintroduce the gap.

describe("fullstack-b-29: TerminalTab WebGL renderer", () => {
  test("imports WebglAddon from @xterm/addon-webgl", () => {
    expect(tab).toMatch(/from\s+"@xterm\/addon-webgl"/);
    expect(tab).toMatch(/import\s*\{\s*WebglAddon\s*\}/);
  });

  test("constructs a WebglAddon instance and loads it onto the terminal", () => {
    expect(tab).toMatch(/new WebglAddon\(\)/);
    expect(tab).toMatch(/term\.loadAddon\(webgl\)/);
  });

  test("registers onContextLoss handler to dispose the addon", () => {
    // WebGL contexts can be lost (GPU reset, tab backgrounding
    // on some platforms). The handler disposes the addon so
    // xterm.js falls back to its DOM rendering for that
    // session instead of crashing.
    expect(tab).toMatch(/webgl\.onContextLoss\(/);
    expect(tab).toMatch(/webglRendererActive\s*=\s*false/);
    expect(tab).toMatch(/webgl\.dispose\(\)/);
  });

  test("wraps load in try/catch with DOM-fallback warning", () => {
    // Headless test harnesses + rare GPU setups can throw on
    // `new WebglAddon()`. The try/catch keeps terminal mount
    // working with the original DOM renderer rather than
    // exploding the panel.
    expect(tab).toMatch(/try\s*\{[\s\S]*?new WebglAddon[\s\S]*?\}\s*catch/);
    expect(tab).toMatch(/falling back to DOM/);
  });

  test("refreshes the WebGL texture atlas after animated SGR output", () => {
    // `fullstack-a-97`: xterm.js documents clearTextureAtlas as
    // the workaround for corrupted WebGL texture atlases. Claude
    // Code's animated ANSI status lines churn SGR color/style
    // sequences; coalescing an atlas clear on those chunks keeps
    // the renderer from substituting glyphs across panes.
    expect(tab).toMatch(/function bytesContainSgrSequence\(bytes: Uint8Array\)/);
    expect(tab).toMatch(/if \(b === 0x6d\) return true/);
    expect(tab).toMatch(/function maybeRefreshWebglAtlas\(bytes: Uint8Array\)/);
    expect(tab).toMatch(/term\?\.clearTextureAtlas\(\)/);
    expect(tab).toMatch(/term\?\.refresh\(0, Math\.max\(0, term\.rows - 1\)\)/);
  });

  test("checks ArrayBuffer and Blob terminal output for SGR atlas refresh", () => {
    expect(tab).toMatch(
      /event\.data instanceof ArrayBuffer[\s\S]*?term\?\.write\(bytes\);[\s\S]*?maybeRefreshWebglAtlas\(bytes\);/,
    );
    expect(tab).toMatch(
      /event\.data instanceof Blob[\s\S]*?term\?\.write\(bytes\);[\s\S]*?maybeRefreshWebglAtlas\(bytes\);/,
    );
  });
});
