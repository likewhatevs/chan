// Raw-source assertions on the Cmd+K (pane mode) keymap wired in
// App.svelte. The dispatcher lives inline inside the App component
// and is hard to mount in isolation, so this gate reads the source
// directly and checks that the arrow → move-focus and WASD → swap
// arrangement from `fullstack-40` is preserved. Catches accidental
// regressions if someone "fixes" the mapping back to the
// `fullstack-16` defaults.

import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";

describe("Cmd+K pane mode keymap (fullstack-40 inversion)", () => {
  test("arrow keys move focus", () => {
    expect(app).toContain('case "ArrowUp":\n        paneModeMoveFocus("up");');
    expect(app).toContain('case "ArrowLeft":\n        paneModeMoveFocus("left");');
    expect(app).toContain('case "ArrowDown":\n        paneModeMoveFocus("down");');
    expect(app).toContain('case "ArrowRight":\n        paneModeMoveFocus("right");');
  });

  test("WASD swaps tiles (lowercase + uppercase, except 's')", () => {
    expect(app).toContain('case "w":\n      case "W":\n        paneModeSwap("up");');
    expect(app).toContain('case "a":\n      case "A":\n        paneModeSwap("left");');
    expect(app).toContain('case "d":\n      case "D":\n        paneModeSwap("right");');
    // `fullstack-42` reassigned lowercase `s` to Search overlay,
    // so only the Shift-modified `S` keeps the swap-down meaning.
    expect(app).toContain('case "S":\n        paneModeSwap("down");');
  });
});

describe("Cmd+K pane mode keymap (fullstack-42 — search / graph / new file / help)", () => {
  test("3 spawns Graph; lowercase s opens the Search overlay", () => {
    // `fullstack-43`: every spawn key now passes a context derived
    // from the focused tab so the new tab anchors on the source's
    // directory / file instead of the drive root.
    expect(app).toContain(
      'case "3":\n        paneModeOpenGraph(resolveSpawnContext());',
    );
    expect(app).toMatch(
      /case "s":[\s\S]*commitPaneMode\(\);[\s\S]*searchPanel\.open = true;/,
    );
  });

  test("4 commits the draft and opens the new-file dialog at the contextual dir", () => {
    expect(app).toMatch(
      /case "4": \{[\s\S]*?const ctx = resolveSpawnContext\(\);[\s\S]*?commitPaneMode\(\);[\s\S]*?fileOps\.createFile\(ctx\.dir\);/,
    );
  });

  test("h toggles the Pane Mode help overlay without committing", () => {
    expect(app).toContain(
      'case "h":\n      case "H":\n        paneModeHelpVisible = !paneModeHelpVisible;',
    );
    // The help block must not call commitPaneMode or
    // scheduleSessionSave near its own case — it's a read-only
    // affordance on top of the live draft.
    expect(app).toMatch(
      /case "h":[\s\S]*?case "H":[\s\S]*?paneModeHelpVisible = !paneModeHelpVisible;\s*\n\s*return;/,
    );
  });
});

describe("Cmd+K pane mode spawn context (fullstack-43)", () => {
  test("1 spawns Terminal with the resolved context", () => {
    expect(app).toContain(
      'case "1":\n        paneModeOpenTerminal(resolveSpawnContext());',
    );
  });

  test("2 primes the browser selection then spawns the File Browser", () => {
    expect(app).toMatch(
      /case "2": \{[\s\S]*?const ctx = resolveSpawnContext\(\);[\s\S]*?if \(ctx\.file\) revealAndSelect\(ctx\.file\);[\s\S]*?else if \(ctx\.dir\) revealAndSelect\(ctx\.dir\);[\s\S]*?paneModeOpenBrowser\(ctx\);/,
    );
  });
});

describe("Cmd+K pane mode rich-prompt binding (fullstack-50)", () => {
  test("p commits the draft and shows/spawns the rich prompt on the focused pane", () => {
    // Commit-first ordering matches the `4` (new file) path so a
    // spawned terminal survives any pane-mode rollback; the actual
    // terminal lookup / spawn lives in
    // showOrSpawnRichPromptInFocusedPane.
    expect(app).toMatch(
      /case "p":[\s\S]*?case "P":[\s\S]*?commitPaneMode\(\);[\s\S]*?showOrSpawnRichPromptInFocusedPane\(\);/,
    );
  });
});

describe("Pane Mode entry flash (fullstack-61)", () => {
  test("Pane Mode active transition triggers a one-shot flash overlay", () => {
    // The `$effect` watches `paneMode.active`; on false → true it
    // bumps `paneModeFlashKey` (so `{#key}` re-triggers the CSS
    // animation) and schedules the auto-dismiss timer. False on
    // exit is intentionally a no-op — only entry triggers.
    expect(app).toMatch(
      /active && !paneModeWasActive[\s\S]*?paneModeFlashKey \+= 1[\s\S]*?paneModeFlashVisible = true[\s\S]*?setTimeout\(\(\) => \{[\s\S]*?paneModeFlashVisible = false/,
    );
  });

  test("flash renders an H key chip plus 'for help' text, non-blocking", () => {
    expect(app).toMatch(
      /\{#if paneModeFlashVisible\}[\s\S]*?\{#key paneModeFlashKey\}[\s\S]*?class="pane-mode-flash"[\s\S]*?<span class="pane-mode-flash-key">H<\/span>[\s\S]*?for help/,
    );
    // pointer-events:none in the CSS so keystrokes (including H,
    // Enter, Esc) flow straight to the existing Pane Mode handlers.
    expect(app).toContain("pointer-events: none");
  });
});
