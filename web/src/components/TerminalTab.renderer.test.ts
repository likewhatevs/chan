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

  test("keeps the event-driven row-repaint helper wired", () => {
    // `refreshTerminalRows` is the event-driven renderer-refresh
    // primitive (mount / focus / blur / host-resume). `desktop-fixes`
    // dropped the companion `clearTextureAtlas` helper - see the
    // negative pin in the next test.
    expect(tab).toMatch(/function refreshTerminalRows\(\): void/);
    expect(tab).toMatch(/maybeRefresh\?\.call\(term, 0, Math\.max\(0, term\.rows - 1\)\)/);
  });

  test("never clears the shared WebGL texture atlas on a per-pane event", () => {
    // `desktop-fixes`: clearing xterm.js's process-global TextureAtlas
    // from one pane's focus / blur / active / wake recovery rebuilt the
    // atlas out from under the sibling panes still on screen, garbling
    // their glyphs when the user moved focus around the grid. The
    // addon-webgl 0.19 renderer rebuilds the atlas itself for
    // color / DPR / font changes, so we never call it manually anymore.
    expect(tab).not.toMatch(/clearTextureAtlas/);
  });

  test("does not clear the texture atlas per PTY data chunk", () => {
    // `fullstack-a-97` removed: the old per-frame SGR-triggered atlas
    // clear force-repainted every terminal pane sharing the addon's
    // process-global TextureAtlas (~60x/sec under animated TUIs),
    // which is itself the source of the cross-pane glyph glitches.
    // xterm 6.0.0 / addon-webgl 0.19.0 handle color/DPR/options
    // changes natively, so the workaround is gone. This pin keeps it
    // from silently returning.
    expect(tab).not.toMatch(/maybeRefreshWebglAtlas/);
    expect(tab).not.toMatch(/bytesContainSgrSequence/);
  });

  test("passes binary terminal output to xterm without string coercion", () => {
    // UTF-8 sequences such as U+2014 must reach xterm as bytes.
    // Coercing ArrayBuffer or Blob output through String() before
    // write would corrupt non-ASCII glyphs.
    expect(tab).toMatch(
      /event\.data instanceof ArrayBuffer[\s\S]*?const bytes = new Uint8Array\(event\.data\);[\s\S]*?writePtyOutput\(bytes\);/,
    );
    expect(tab).toMatch(
      /event\.data instanceof Blob[\s\S]*?const bytes = new Uint8Array\(await event\.data\.arrayBuffer\(\)\);[\s\S]*?writePtyOutput\(bytes\);/,
    );
    expect(tab).not.toMatch(/term\?\.write\(String\(event\.data\)\)/);
  });

  test("refreshes renderer on focus and after font readiness", () => {
    expect(tab).toMatch(/function refreshTerminalRenderer\(\): void/);
    // `desktop-fixes`: repaint-only - rAF and fonts.ready each call
    // refreshTerminalRows(); no texture-atlas clear (see the negative
    // pin above).
    expect(tab).toMatch(
      /requestAnimationFrame\([\s\S]*?refreshTerminalRows\(\);[\s\S]*?\}\);/,
    );
    expect(tab).toMatch(/document\.fonts\?\.ready\.then/);
    // `desktop-fixes`: the focus-GAIN path runs the full host-resume
    // recovery (not a bare queueFit + refreshTerminalRenderer) so the
    // pane focused after another was active repaints clean in WKWebView
    // instead of showing stale glyphs.
    expect(tab).toMatch(
      /if \(!focused\) return;[\s\S]*?recoverTerminalRendererAfterHostResume\(\);[\s\S]*?setTerminalActivity\(tab, false\);/,
    );
    // `lane-c addendum-1 bug 1`: the blur effect runs the full
    // host-resume recovery (fit + repaint + delayed re-fits), not a
    // bare refreshTerminalRenderer, so the pane LOSING focus repaints
    // clean in WKWebView (the desktop app) where a single refresh
    // leaves it stale.
    expect(tab).toMatch(
      /if \(focused\) return;[\s\S]*?recoverTerminalRendererAfterHostResume\(\);[\s\S]*?sendFocusState\(\);/,
    );
  });

  test("refreshes renderer after native host resume", () => {
    expect(tab).toMatch(/function recoverTerminalRendererAfterHostResume\(\): void/);
    expect(tab).toMatch(/clearHostResumeTimers\(\);[\s\S]*?queueFit\(\);[\s\S]*?refreshTerminalRenderer\(\);/);
    expect(tab).toMatch(/for \(const delay of \[50, 250\]\)/);
    expect(tab).toMatch(/window\.addEventListener\("focus", onHostResume\)/);
    expect(tab).toMatch(/window\.addEventListener\("pageshow", onHostResume\)/);
    expect(tab).toMatch(/document\.addEventListener\("visibilitychange", onVisibility\)/);
    expect(tab).toMatch(/frame\.type === "ready"[\s\S]*?recoverTerminalRendererAfterHostResume\(\);/);
    expect(tab).toMatch(/hostResumeListenerCleanup\?\.\(\)/);
  });

  test("a wall-clock-gap wake probe fires recovery on sleep/wake (lane-c addendum-2 item 2)", () => {
    // macOS sleep doesn't fire focus/pageshow/visibilitychange in
    // WKWebView, so a coarse interval detects the wake (timers froze ->
    // the callback fires far later than scheduled) and runs the same
    // recovery. Source-pinned (the timer-gap is not deterministically
    // unit-testable without mounting xterm + faking sleep).
    expect(tab).toMatch(/wakeProbeTimer = setInterval\(/);
    expect(tab).toMatch(/if \(gap > WAKE_GAP_MS\) recoverTerminalRendererAfterHostResume\(\);/);
    // Torn down with the host-resume listeners.
    expect(tab).toMatch(/clearInterval\(wakeProbeTimer\)/);
  });

  test("prefers server-provided virtual cwd when present", () => {
    expect(tab).toMatch(/cwd_rel\?: string \| null/);
    expect(tab).toMatch(/terminalCwdVirtual = frame\.cwd_rel \?\? null/);
    expect(tab).toMatch(/if \(terminalCwdVirtual !== null\) return terminalCwdVirtual/);
  });
});
