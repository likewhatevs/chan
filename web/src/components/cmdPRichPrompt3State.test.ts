import { describe, expect, test } from "vitest";
import tabs from "../state/tabs.svelte.ts?raw";
import graph from "./GraphPanel.svelte?raw";

// `fullstack-a-56` — bundled UX papercut:
//
// 1. Cmd+P 3-state contract in `showOrSpawnRichPromptInFocusedPane`:
//    pre-`-a-56` the function picked the FIRST terminal in the
//    pane (`p.tabs.find((t) => t.kind === "terminal")`) regardless
//    of which tab was active + always set richPrompt.open = true
//    (no toggle-off path). Rewrite reads p.activeTabId + branches
//    on (terminal vs not) and (open vs closed).
//
// 2. Depth slider shallow-scope cue: when depthCap <= 1, render
//    a `[max]` suffix so the user sees the slider is already at
//    max without needing to drag.

describe("fullstack-a-56 Cmd+P 3-state contract", () => {
  test("function reads p.activeTabId rather than picking the first terminal", () => {
    // Pre-`-a-56` shape (removed):
    //   const terminal = p.tabs.find((t) => t.kind === "terminal");
    // New shape: look up the ACTIVE tab + branch on its kind.
    expect(tabs).toMatch(
      /export function showOrSpawnRichPromptInFocusedPane\(\): void \{[\s\S]*?const activeTab = p\.tabs\.find\(\(t\) => t\.id === p\.activeTabId\);/,
    );
  });

  test("case 1: active terminal + prompt closed → open on current", () => {
    expect(tabs).toMatch(
      /if \(activeTab\?\.kind === "terminal"\) \{[\s\S]*?if \(activeTab\.richPrompt\?\.open\) \{[\s\S]*?\}[\s\S]*?openActiveTerminalRichPrompt\(\);/,
    );
  });

  test("case 2: active terminal + prompt open → toggle off (richPrompt.open = false)", () => {
    expect(tabs).toMatch(
      /if \(activeTab\.richPrompt\?\.open\) \{[\s\S]*?activeTab\.richPrompt\.open = false;[\s\S]*?return;/,
    );
  });

  test("case 3: active tab not a terminal → spawn fresh + open", () => {
    // After the terminal branch returns, the fallback path spawns
    // a fresh terminal in the pane then opens the rich prompt on
    // the newly active tab.
    expect(tabs).toMatch(
      /openTerminalInPane\(p\.id, \{\}\);\s*\n\s*openActiveTerminalRichPrompt\(\);/,
    );
  });

  test("pre-`-a-56` first-terminal lookup is gone (no `p.tabs.find((t): t is TerminalTab => t.kind === \"terminal\")`)", () => {
    expect(tabs).not.toMatch(
      /const terminal = p\.tabs\.find\(\s*\(t\): t is TerminalTab => t\.kind === "terminal",?\s*\);/,
    );
  });
});

describe("fullstack-a-56 depth-slider shallow-scope cue", () => {
  test("derived depthShallow gate: only for non-language non-disabled scopes where depthCap <= 1", () => {
    // Hoisted as a top-level `$derived.by` so it can drive both
    // the `disabled` attribute on the slider AND the markup
    // cue without invoking `{@const}` outside an allowed
    // parent block.
    expect(graph).toMatch(
      /const depthShallow = \$derived\.by\(\(\) => \{[\s\S]*?if \(languageMode\) return false;[\s\S]*?return depthCap <= 1;/,
    );
  });

  test("slider input is disabled when shallow (no point dragging)", () => {
    expect(graph).toMatch(/disabled=\{depthDisabled \|\| depthShallow\}/);
  });

  test("depth-row gets a .shallow class + tooltip when shallow", () => {
    expect(graph).toMatch(/class:shallow=\{depthShallow\}/);
    expect(graph).toMatch(
      /title=\{depthShallow[\s\S]*?Scope is shallow[\s\S]*?depth 1 already reveals/,
    );
  });

  test("depth value renders `[max]` cue when shallow", () => {
    expect(graph).toMatch(
      /\{:else if depthShallow\}[\s\S]*?\{graphState\.depth\} <span class="depth-cue">\[max\]<\/span>/,
    );
  });

  test("CSS rule for .depth-cue exists (dimmer + smaller than the numeric value)", () => {
    expect(graph).toMatch(/\.tab-menu-bubble \.depth-cue \{[\s\S]*?opacity: 0\.7;/);
  });
});
