import { describe, expect, test } from "vitest";
import tab from "./TerminalTab.svelte?raw";

// TerminalTab uses the WebGL renderer. The DOM renderer renders
// box-drawing characters via the system font, leaving vertical gaps at
// lineHeight: 1.2. The WebGL renderer's customGlyphs path fills the
// entire cell including line-height padding. These pins guard the wiring.

describe("TerminalTab WebGL renderer", () => {
  test("imports WebglAddon from @xterm/addon-webgl", () => {
    expect(tab).toMatch(/from\s+"@xterm\/addon-webgl"/);
    expect(tab).toMatch(/import\s*\{\s*WebglAddon\s*\}/);
  });

  test("constructs a WebglAddon instance and loads it onto the terminal", () => {
    expect(tab).toMatch(/new WebglAddon\(\)/);
    expect(tab).toMatch(/term\.loadAddon\(webgl\)/);
  });

  test("registers onContextLoss handler to dispose the addon", () => {
    // GPU reset or tab backgrounding can lose the WebGL context.
    // Disposing the addon lets xterm.js fall back to DOM rendering.
    expect(tab).toMatch(/webgl\.onContextLoss\(/);
    expect(tab).toMatch(/webglRendererActive\s*=\s*false/);
    expect(tab).toMatch(/webgl\.dispose\(\)/);
  });

  test("wraps load in try/catch with DOM-fallback warning", () => {
    // Headless harnesses and rare GPU setups may throw on new WebglAddon().
    // The try/catch keeps terminal mount working with the DOM renderer.
    expect(tab).toMatch(/try\s*\{[\s\S]*?new WebglAddon[\s\S]*?\}\s*catch/);
    expect(tab).toMatch(/falling back to DOM/);
  });

  test("keeps the event-driven row-repaint helper wired", () => {
    // refreshTerminalRows is the renderer-refresh primitive for mount /
    // focus / blur / host-resume events.
    expect(tab).toMatch(/function refreshTerminalRows\(\): void/);
    expect(tab).toMatch(/maybeRefresh\?\.call\(term, 0, Math\.max\(0, term\.rows - 1\)\)/);
  });

  test("never clears the shared WebGL texture atlas on a per-pane event", () => {
    // Clearing the process-global TextureAtlas from one pane's focus/blur
    // rebuilt it under sibling panes, garbling their glyphs. The renderer
    // handles color/DPR/font changes itself, so manual clears are removed.
    expect(tab).not.toMatch(/clearTextureAtlas/);
  });

  test("does not clear the texture atlas per PTY data chunk", () => {
    // The old per-frame SGR-triggered atlas clear force-repainted all
    // terminal panes (~60x/sec under animated TUIs). The renderer handles
    // changes natively, so the workaround is gone.
    expect(tab).not.toMatch(/maybeRefreshWebglAtlas/);
    expect(tab).not.toMatch(/bytesContainSgrSequence/);
  });

  test("passes binary terminal output to xterm without string coercion", () => {
    // UTF-8 sequences must reach xterm as bytes. Coercing through
    // String() would corrupt non-ASCII glyphs.
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
    // rAF and fonts.ready each call refreshTerminalRows(); no atlas
    // clear (see the negative pin above).
    expect(tab).toMatch(
      /requestAnimationFrame\([\s\S]*?refreshTerminalRows\(\);[\s\S]*?\}\);/,
    );
    expect(tab).toMatch(/document\.fonts\?\.ready\.then/);
    // Focus-gain runs full host-resume recovery so a pane focused after
    // another repaints clean in WKWebView rather than showing stale glyphs.
    expect(tab).toMatch(
      /if \(!focused\) return;[\s\S]*?recoverTerminalRendererAfterHostResume\(\);[\s\S]*?setTerminalActivity\(tab, false\);/,
    );
    // Blur also runs full host-resume recovery so the pane LOSING focus
    // repaints clean in WKWebView (a single refresh leaves it stale there).
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

  test("a wall-clock-gap wake probe fires recovery on sleep/wake", () => {
    // macOS sleep does not fire focus/pageshow/visibilitychange in
    // WKWebView. A coarse interval detects the wake by observing that
    // the timer callback fired far later than scheduled.
    expect(tab).toMatch(/wakeProbeTimer = setInterval\(/);
    expect(tab).toMatch(/if \(gap > WAKE_GAP_MS\) recoverTerminalRendererAfterHostResume\(\);/);
    expect(tab).toMatch(/clearInterval\(wakeProbeTimer\)/);
  });

  test("prefers server-provided virtual cwd when present", () => {
    expect(tab).toMatch(/cwd_rel\?: string \| null/);
    expect(tab).toMatch(/terminalCwdVirtual = frame\.cwd_rel \?\? null/);
    expect(tab).toMatch(/if \(terminalCwdVirtual !== null\) return terminalCwdVirtual/);
  });
});
