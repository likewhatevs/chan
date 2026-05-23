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

  test("WASD swaps tiles (lowercase + uppercase, including 's')", () => {
    expect(app).toContain('case "w":\n      case "W":\n        paneModeSwap("up");');
    expect(app).toContain('case "a":\n      case "A":\n        paneModeSwap("left");');
    expect(app).toContain('case "d":\n      case "D":\n        paneModeSwap("right");');
    // `fullstack-74`: `s` rejoins WASD. Search moved to `f`.
    expect(app).toContain('case "s":\n      case "S":\n        paneModeSwap("down");');
  });
});

// `fullstack-a-32` reshaped the Hybrid NAV spawn cases. The numeric
// 1/2/3/4 cases are gone (they duplicated the new top-level chord
// set Cmd+T / Cmd+O / Cmd+P / Cmd+Shift+M). The letter mnemonics
// `t/T` (terminal), `o/O` (browser), `p/P` (rich prompt — kept
// from -50), `v/V` (graph) are the in-Hybrid-NAV path; each
// commits immediately and routes through the same context-aware
// helper as the matching top-level chord. `f/F` (Search) and
// `h/H` (Help) are unchanged.
describe("Cmd+K pane mode keymap (fullstack-a-68 slice 2 — transactional staging)", () => {
  test("g / G writes directly to the draft layout (no immediate commit)", () => {
    expect(app).toMatch(
      /case "g":\s*\n\s*case "G":\s*\n?\s*case "v":\s*\n\s*case "V":[\s\S]*?paneModeOpenGraph\(resolveSpawnContext\(\)\);[\s\S]*?return;/,
    );
    // v / V kept as legacy aliases — muscle memory protection.
    expect(app).toMatch(/case "v":\s*\n\s*case "V":[\s\S]*?paneModeOpenGraph/);
  });

  test("lowercase f opens the Search overlay (commits first)", () => {
    expect(app).toMatch(
      /case "f":[\s\S]*?case "F":[\s\S]*?commitPaneMode\(\);[\s\S]*?searchPanel\.open = true;/,
    );
  });

  test("h toggles the Pane Mode help overlay without committing", () => {
    expect(app).toContain(
      'case "h":\n      case "H":\n        paneModeHelpVisible = !paneModeHelpVisible;',
    );
  });

  test("numeric 1 / 2 / 3 / 4 cases stay gone", () => {
    // pre-`-a-32` cleanup preserved.
    expect(app).not.toMatch(/case "1": \{[\s\S]{0,60}paneModeStageSpawn/);
    expect(app).not.toMatch(/case "2": \{[\s\S]{0,60}paneModeStageSpawn/);
    expect(app).not.toMatch(/case "3": \{[\s\S]{0,60}paneModeStageSpawn/);
    expect(app).not.toMatch(/case "4": \{[\s\S]{0,60}fileOps\.createFile/);
  });
});

describe("Cmd+K pane mode transactional staging (fullstack-a-68 slice 2)", () => {
  test("t / T stages a terminal write into the draft (no immediate commit)", () => {
    expect(app).toMatch(
      /case "t":\s*\n\s*case "T":\s*\n?\s*paneModeOpenTerminal\(resolveSpawnContext\(\)\);\s*\n\s*return;/,
    );
    // No commitPaneMode inside the t/T branch.
    expect(app).not.toMatch(
      /case "t":\s*\n\s*case "T":\s*\n?\s*paneModeOpenTerminal\(resolveSpawnContext\(\)\);\s*\n\s*commitPaneMode/,
    );
  });

  test("o / O stages a browser + primes browserSelection without committing", () => {
    expect(app).toMatch(
      /case "o":\s*\n\s*case "O": \{[\s\S]*?const ctx = resolveSpawnContext\(\);[\s\S]*?paneModeOpenBrowser\(ctx\);[\s\S]*?revealAndSelect\(ctx\.file\);[\s\S]*?revealAndSelect\(ctx\.dir\);[\s\S]*?return;\s*\n\s*\}/,
    );
  });

  test("p / P stages a rich-prompt terminal (no immediate commit, no toggle on existing terminal)", () => {
    expect(app).toMatch(
      /case "p":\s*\n\s*case "P":\s*\n?\s*paneModeOpenRichPromptTerminal\(resolveSpawnContext\(\)\);\s*\n\s*return;/,
    );
  });

  test("n / N stages a new draft editor onto the focused pane", () => {
    expect(app).toMatch(
      /case "n":\s*\n\s*case "N":\s*\n?\s*paneModeStageDraftEditor\(\);\s*\n\s*return;/,
    );
  });

  test("Enter materializes staged draft editors before commit", () => {
    expect(app).toMatch(
      /case "Enter": \{[\s\S]*?materializeStagedDraftEditors\(\);[\s\S]*?commitPaneMode\(\);/,
    );
  });

  test("Escape cancels (no materializeStagedDraftEditors call before cancelPaneMode)", () => {
    // Cancel must run BEFORE materialize would — verify no
    // materialize call sits between Enter's branch and the
    // Escape case's cancelPaneMode (i.e. Escape just cancels).
    expect(app).toMatch(
      /case "Escape":[\s\S]{0,800}cancelPaneMode\(\);/,
    );
    // Sanity: the Escape branch must NOT call
    // materializeStagedDraftEditors (it's OK to mention it in
    // a comment for context — only the `()` call form is
    // forbidden, and only before cancelPaneMode runs).
    expect(app).not.toMatch(
      /case "Escape":[\s\S]{0,400}materializeStagedDraftEditors\(\);[\s\S]{0,400}cancelPaneMode\(\);/,
    );
  });
});

describe("Cmd+T / O / P / Cmd+Shift+M top-level chords (fullstack-a-32)", () => {
  test("Cmd+Alt+T (web Mac) routes through context-aware spawn helper", () => {
    // pre-`-a-32` handler called `openTerminalInActivePane()` with
    // no args. -a-32 routes through `spawnTerminalFromContext`
    // which threads `resolveSpawnContext().dir` as `cwd`.
    expect(app).toMatch(
      /e\.metaKey && e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyT"[\s\S]*?spawnTerminalFromContext\(\)/,
    );
  });

  test("Cmd+Alt+O (web Mac) spawns file browser with context", () => {
    expect(app).toMatch(
      /e\.metaKey && e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyO"[\s\S]*?spawnBrowserFromContext\(\)/,
    );
  });

  test("Cmd+Alt+P (web Mac) spawns rich prompt with context", () => {
    expect(app).toMatch(
      /e\.metaKey && e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyP"[\s\S]*?spawnRichPromptFromContext\(\)/,
    );
  });

  test("Cmd+Shift+M (web + native) spawns graph with context", () => {
    expect(app).toMatch(
      /e\.metaKey && !e\.altKey && e\.shiftKey && !e\.ctrlKey && e\.code === "KeyM"[\s\S]*?spawnGraphFromContext\(\)/,
    );
  });

  test("chan:command bridge routes through context-aware helpers", () => {
    // chan-desktop's KEY_BRIDGE_JS fires `app.terminal.toggle` /
    // `app.files.toggle` / `app.terminal.richPrompt` /
    // `app.graph.toggle` on native Cmd+T / Cmd+O / Cmd+P /
    // Cmd+Shift+M. -a-32 routes them through the same helpers
    // the web chords use so native + web behave identically.
    expect(app).toMatch(
      /case "app\.terminal\.toggle":\s*\n\s*spawnTerminalFromContext\(\);/,
    );
    expect(app).toMatch(
      /case "app\.files\.toggle":\s*\n\s*spawnBrowserFromContext\(\);/,
    );
    expect(app).toMatch(
      /case "app\.terminal\.richPrompt":\s*\n\s*spawnRichPromptFromContext\(\);/,
    );
    expect(app).toMatch(
      /case "app\.graph\.toggle":\s*\n\s*spawnGraphFromContext\(\);/,
    );
  });
});

describe("Cmd+K pane mode rich-prompt binding (fullstack-a-68 slice 2 — retire fullstack-50 toggle)", () => {
  test("p inside Hybrid Nav stages a smart-prompt terminal (no commit, no toggle on existing)", () => {
    // pre-`-a-68 slice 2` (`fullstack-50`): `P` committed the
    // draft + showed/spawned the rich prompt on the focused
    // pane's existing terminal. Addendum-a's "back to
    // transactional mode" framing replaces that with a fresh
    // smart-prompt terminal staged into the draft.
    expect(app).toMatch(
      /case "p":\s*\n\s*case "P":\s*\n?\s*paneModeOpenRichPromptTerminal\(resolveSpawnContext\(\)\);/,
    );
    // The legacy "commit then show" path no longer surfaces
    // inside the Hybrid Nav P case.
    expect(app).not.toMatch(
      /case "p":\s*\n\s*case "P":\s*\n?\s*commitPaneMode\(\);\s*\n[\s\S]{0,200}showOrSpawnRichPromptInFocusedPane/,
    );
  });

  test("showOrSpawnRichPromptInFocusedPane still imported (top-level Cmd+P chord uses it)", () => {
    // The toggle behavior remains reachable from outside
    // Hybrid Nav via `spawnRichPromptFromContext` →
    // `showOrSpawnRichPromptInFocusedPane`.
    expect(app).toMatch(
      /import \{[\s\S]{1,4000}showOrSpawnRichPromptInFocusedPane,/,
    );
  });
});

describe("Cmd+K Backspace kill-pane (fullstack-77)", () => {
  test("Backspace closes the focused pane; k no longer bound to kill-pane", () => {
    // `fullstack-77`: kill-pane moved from `k` / `K` to
    // `Backspace`. The old letter is unbound (not repurposed) so
    // the previous case block disappears from the dispatch.
    expect(app).toMatch(
      /case "Backspace":[\s\S]*?commitPaneMode\(\);[\s\S]*?closePane\(layout\.activePaneId\);/,
    );
    expect(app).not.toMatch(
      /case "k":\s*\n\s*case "K":\s*\n\s*commitPaneMode\(\);[\s\S]*?closePane\(layout\.activePaneId\);/,
    );
  });
});

describe("Cmd+K dock toggles (fullstack-69)", () => {
  test("< toggles the right-side file browser dock", () => {
    // Mapping per @@Alex's verbatim spec: less-than (right-facing
    // arrow when read as an opening tag) toggles the right dock.
    expect(app).toMatch(
      /case "<":[\s\S]*?commitPaneMode\(\);[\s\S]*?toggleBrowserSidePane\("right"\);/,
    );
  });

  test("> toggles the left-side file browser dock", () => {
    expect(app).toMatch(
      /case ">":[\s\S]*?commitPaneMode\(\);[\s\S]*?toggleBrowserSidePane\("left"\);/,
    );
  });
});

describe("Pane Mode entry flash removed (fullstack-a-3)", () => {
  test("no centre-window 'H for help' flash on Pane Mode entry", () => {
    // `fullstack-61` introduced the flash; `fullstack-a-3` removes
    // it as visual noise — the status-bar Hybrid pill already
    // telegraphs `H help` and PaneModeHelp covers discovery.
    expect(app).not.toContain("pane-mode-flash");
    expect(app).not.toContain("paneModeFlashVisible");
    expect(app).not.toContain("paneModeFlashKey");
    expect(app).not.toContain("PANE_MODE_FLASH_MS");
  });
});
