import { describe, expect, test } from "vitest";
import tabs from "../state/tabs.svelte.ts?raw";
import graph from "./GraphPanel.svelte?raw";

// Phase 9 Rich Prompt entrypoint check:
//
// 1. Cmd+P always creates a fresh terminal with Rich Prompt open.
//    It no longer toggles a prompt on the current terminal.
//
// 2. Depth slider shallow-scope cue: when depthCap <= 1, render
//    a `[max]` suffix so the user sees the slider is already at
//    max without needing to drag.

describe("Phase 9 Cmd+P fresh Rich Prompt terminal", () => {
  test("helper always opens a new terminal in the active pane", () => {
    expect(tabs).toMatch(
      /export function showOrSpawnRichPromptInFocusedPane\([\s\S]*?opts: OpenTerminalOptions = \{\},[\s\S]*?\): void \{[\s\S]*?const p = activePane\(\);[\s\S]*?openTerminalInPane\(p\.id, opts\);[\s\S]*?openActiveTerminalRichPrompt\(\);/,
    );
  });

  test("old active-terminal toggle branch is gone", () => {
    expect(tabs).not.toMatch(/activeTab\.richPrompt\?\.open/);
    expect(tabs).not.toMatch(/richPrompt\.open = false/);
  });
});

describe("fullstack-a-56 depth-slider shallow-scope cue", () => {
  test("derived depthShallow gate: only for non-language non-disabled scopes where depthCap <= 1", () => {
    // Hoisted as a top-level `$derived.by` so it can workspace both
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
