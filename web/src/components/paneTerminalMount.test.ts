import { describe, expect, test } from "vitest";
import pane from "./Pane.svelte?raw";

// `fullstack-b-2`: pane mode (Hybrid NAV / Cmd+K) used to unmount
// every TerminalTab in the pane, disposing the xterm.js EditorView
// and dropping the 20k-line scrollback buffer. The active terminal
// now stays mounted; only its `active` prop flips to false during
// pane mode so the existing CSS rule hides it via
// `visibility: hidden`. These checks pin the source so a future
// edit that re-introduces the outer `{#if !paneMode.active}`
// wrapper around the terminal each-block trips the test.

describe("fullstack-b-2: TerminalTabs survive Hybrid NAV toggles", () => {
  test("terminal each-block does not sit under a {#if !paneMode.active}", () => {
    // The pre-fix pattern was: `{#if !paneMode.active}` immediately
    // followed by the terminal each-block, with only whitespace and
    // {/if} between them. Asserting the absence of that exact
    // adjacency catches the regression without false-matching the
    // separate `{#if paneMode.active}` block above (which renders
    // the pane-mode-preview).
    const banned = /\{#if\s+!paneMode\.active\}\s+\{#each pane\.tabs\.filter\(\(t\) => t\.kind === "terminal"\) as t \(t\.id\)\}/;
    expect(pane).not.toMatch(banned);
  });

  test("active prop is gated by !paneMode.active", () => {
    // The prop must short-circuit on pane mode so the existing
    // visibility-hidden CSS rule fires during Hybrid NAV.
    expect(pane).toMatch(/active=\{!paneMode\.active && t\.id === pane\.activeTabId\}/);
  });

  test("focused prop is gated by !paneMode.active", () => {
    // Same gate on focused so we don't pull focus into a hidden
    // xterm during pane mode (would swallow the next chord).
    expect(pane).toMatch(/focused=\{!paneMode\.active && t\.id === pane\.activeTabId && viewLayout\.activePaneId === pane\.id\}/);
  });
});
