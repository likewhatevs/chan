// Raw-source assertions on the Cmd+K pane-mode keymap in App.svelte.
// The dispatcher is hard to mount in isolation, so the source is read
// directly. Pins the arrow = move-focus and WASD = swap arrangement.

import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";

describe("Cmd+K pane mode keymap (inversion)", () => {
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
    // `s` rejoins WASD; Search moved to `f`.
    expect(app).toContain('case "s":\n      case "S":\n        paneModeSwap("down");');
  });
});

// Hybrid NAV spawn cases: numeric 1/2/3/4 are gone (they duplicated
// Cmd+T / Cmd+O / Cmd+P / Cmd+Shift+M). Letter mnemonics t/o/p/v/g
// are the in-NAV path; f/F (Search) and h/H (Help) are unchanged.
describe("Cmd+K pane mode keymap (transactional staging)", () => {
  test("g / G writes directly to the draft layout (no immediate commit)", () => {
    expect(app).toMatch(
      /case "g":\s*\n\s*case "G":\s*\n?\s*case "v":\s*\n\s*case "V":[\s\S]*?paneModeOpenGraph\(resolveSpawnContext\(\)\);[\s\S]*?return;/,
    );
    // v / V kept as legacy aliases - muscle memory protection.
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
    expect(app).not.toMatch(/case "1": \{[\s\S]{0,60}paneModeStageSpawn/);
    expect(app).not.toMatch(/case "2": \{[\s\S]{0,60}paneModeStageSpawn/);
    expect(app).not.toMatch(/case "3": \{[\s\S]{0,60}paneModeStageSpawn/);
    expect(app).not.toMatch(/case "4": \{[\s\S]{0,60}fileOps\.createFile/);
  });
});

describe("Cmd+K pane mode transactional staging", () => {
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

  test("p / P no longer stages a Team Work terminal (decoupled to lead-only)", () => {
    // The pane-mode P binding that spawned a bare Team Work bubble terminal on
    // an arbitrary pane was removed: the bubble renders only on a team LEAD
    // terminal via the Cmd+P workflow, so pane mode no longer references the
    // removed spawn.
    expect(app).not.toMatch(/paneModeOpenTeamWorkTerminal/);
  });

  test("n / N stages a new draft editor onto the focused pane", () => {
    // The terminal-only guard (`if (ui.terminalOnly) return;`) sits between
    // the case labels and the stage call in a `?kind=terminal` window, so
    // allow arbitrary intervening source here.
    expect(app).toMatch(
      /case "n":\s*\n\s*case "N":[\s\S]*?paneModeStageDraftEditor\(\);\s*\n\s*return;/,
    );
  });

  test("Enter materializes staged draft editors before commit", () => {
    expect(app).toMatch(
      /case "Enter": \{[\s\S]*?materializeStagedDraftEditors\(\);[\s\S]*?commitPaneMode\(\);/,
    );
  });

  test("Escape cancels without materializing staged draft editors", () => {
    expect(app).toMatch(
      /case "Escape":[\s\S]{0,800}cancelPaneMode\(\);/,
    );
    // The Escape branch must not call materializeStagedDraftEditors.
    expect(app).not.toMatch(
      /case "Escape":[\s\S]{0,400}materializeStagedDraftEditors\(\);[\s\S]{0,400}cancelPaneMode\(\);/,
    );
  });
});

describe("Cmd+T / O / P / Cmd+Shift+M top-level chords", () => {
  test("Cmd+Alt+T (web Mac) routes through context-aware spawn helper", () => {
    // Routes through spawnTerminalFromContext so resolveSpawnContext().dir
    // is threaded as cwd.
    expect(app).toMatch(
      /e\.metaKey && e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyT"[\s\S]*?spawnTerminalFromContext\(\)/,
    );
  });

  test("Cmd+Alt+O (web Mac) spawns file browser with context", () => {
    expect(app).toMatch(
      /e\.metaKey && e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyO"[\s\S]*?spawnBrowserFromContext\(\)/,
    );
  });

  test("Cmd+Alt+P (web Mac) spawns Team Work with context", () => {
    expect(app).toMatch(
      /e\.metaKey && e\.altKey && !e\.shiftKey && !e\.ctrlKey && e\.code === "KeyP"[\s\S]*?spawnTeamWorkFromContext\(\)/,
    );
  });

  test("Cmd+Shift+M (web + native) spawns graph with context", () => {
    expect(app).toMatch(
      /e\.metaKey && !e\.altKey && e\.shiftKey && !e\.ctrlKey && e\.code === "KeyM"[\s\S]*?spawnGraphFromContext\(\)/,
    );
  });

  test("chan:command bridge routes through context-aware helpers", () => {
    // chan-desktop's KEY_BRIDGE_JS fires these commands on native
    // Cmd+T / Cmd+O / Cmd+P / Cmd+Shift+M and they route through
    // the same helpers as the web chords.
    expect(app).toMatch(
      /case "app\.terminal\.toggle":\s*\n\s*spawnTerminalFromContext\(\);/,
    );
    expect(app).toMatch(
      /case "app\.files\.toggle":\s*\n\s*spawnBrowserFromContext\(\);/,
    );
    expect(app).toMatch(
      /case "app\.terminal\.teamWork":\s*\n\s*spawnTeamWorkFromContext\(\);/,
    );
    expect(app).toMatch(
      /case "app\.graph\.toggle":\s*\n\s*spawnGraphFromContext\(\);/,
    );
  });
});

describe("Cmd+K pane mode Team Work binding", () => {
  test("Hybrid Nav has no P Team Work binding (lead-only via the Cmd+P dialog)", () => {
    // The Hybrid Nav P binding that spawned a bare Team Work bubble terminal
    // was removed when the bubble was decoupled from arbitrary terminals. Team
    // Work is now reached ONLY through the top-level Cmd+P / Cmd+Alt+P dialog
    // (spawnTeamWorkFromContext); pane mode never spawns the bubble.
    expect(app).not.toMatch(/paneModeOpenTeamWorkTerminal/);
  });

  test("top-level Cmd+P flow creates a Team Work lead terminal then opens the dialog", () => {
    // The lead terminal is created first (createTeamWorkLeadTerminal),
    // then the Spawn-agents dialog is opened over it. The old
    // showOrSpawnTeamWorkInFocusedPane helper is gone.
    expect(app).toMatch(/import \{[\s\S]{1,4000}createTeamWorkLeadTerminal,/);
    expect(app).toMatch(
      /function spawnTeamWorkFromContext\(\): void \{[\s\S]*?createTeamWorkLeadTerminal\(\{ cwd: ctx\.dir \}\);[\s\S]*?openTeamDialog\(\{ leadTabId: lead\.id, leadPaneId: activePane\(\)\.id \}\);/,
    );
    expect(app).not.toMatch(/showOrSpawnTeamWorkInFocusedPane/);
  });
});

describe("Cmd+K Backspace kill-pane", () => {
  test("Backspace closes the focused pane; k no longer bound to kill-pane", () => {
    // kill-pane moved to Backspace; the old `k` / `K` binding is gone.
    expect(app).toMatch(
      /case "Backspace":[\s\S]*?commitPaneMode\(\);[\s\S]*?killActivePane\(\);/,
    );
    expect(app).not.toMatch(
      /case "k":\s*\n\s*case "K":\s*\n\s*commitPaneMode\(\);[\s\S]*?killActivePane\(\);/,
    );
  });
});

describe("Track C pane shortcut wiring", () => {
  // Web pane nav uses Alt+[/] because Cmd+[/] is browser back/forward.
  // Desktop-native keeps Cmd+[/] via KEY_BRIDGE_JS.
  test("Alt+[ and Alt+] dispatch previous/next pane on web", () => {
    expect(app).toMatch(
      /e\.altKey && !e\.shiftKey && !meta && e\.code === "BracketLeft"[\s\S]*?selectPrevPane\(\);/,
    );
    expect(app).toMatch(
      /e\.altKey && !e\.shiftKey && !meta && e\.code === "BracketRight"[\s\S]*?selectNextPane\(\);/,
    );
  });

  test("close-all and kill-pane command ids route through transactional helpers", () => {
    expect(app).toMatch(
      /case "app\.pane\.closeTabs":[\s\S]*?closeTabsInActivePane\(\);/,
    );
    expect(app).toMatch(
      /case "app\.pane\.kill":[\s\S]*?killActivePane\(\);/,
    );
    expect(app).toMatch(
      /function closeTabsInActivePane\(\): void \{[\s\S]*?closeTabsInPane\(paneId\)\.then\(\(closed\) => \{[\s\S]*?if \(closed\) scheduleSessionSave\(\);/,
    );
    expect(app).toMatch(
      /function killActivePane\(opts\?: \{ force\?: boolean \}\): void \{[\s\S]*?closePane\(paneId, opts\)\.then\(\(closed\) => \{[\s\S]*?if \(closed\) scheduleSessionSave\(\);/,
    );
  });

  test("empty-pane close is wired through Ctrl+D and app.tab.close", () => {
    expect(app).toMatch(
      /if \(!active\) \{[\s\S]*?if \(closeActiveEmptyPane\(\)\) \{[\s\S]*?e\.preventDefault\(\);[\s\S]*?e\.stopPropagation\(\);/,
    );
    expect(app).toMatch(
      /case "app\.tab\.close": \{[\s\S]*?else closeActiveEmptyPane\(\);/,
    );
  });
});

describe("Cmd+K dock toggles", () => {
  test("< toggles the right-side file browser dock", () => {
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

describe("Pane Mode entry flash removed", () => {
  test("no centre-window 'H for help' flash on Pane Mode entry", () => {
    // The status-bar Hybrid pill and PaneModeHelp cover discovery;
    // the centre-window flash is removed as visual noise.
    expect(app).not.toContain("pane-mode-flash");
    expect(app).not.toContain("paneModeFlashVisible");
    expect(app).not.toContain("paneModeFlashKey");
    expect(app).not.toContain("PANE_MODE_FLASH_MS");
  });
});
