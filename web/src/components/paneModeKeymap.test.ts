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

describe("Cmd+K pane mode keymap (fullstack-42 — search / graph / new file / help)", () => {
  test("3 commits a Graph spawn immediately; lowercase f opens the Search overlay", () => {
    // `fullstack-a-3`: 1/2/3 commit on keypress rather than wait
    // for Enter. The stage call is followed by commitPaneMode()
    // in the same case so the spawn lands on the same press.
    // `fullstack-74` moved Search from `s` (which now rejoins
    // WASD swap-down) to `f` / `F` so WASD can fully own
    // swap-tile.
    expect(app).toMatch(
      /case "3": \{[\s\S]*?paneModeStageSpawn\("graph", resolveSpawnContext\(\)\);[\s\S]*?commitPaneMode\(\);/,
    );
    expect(app).toMatch(
      /case "f":[\s\S]*?case "F":[\s\S]*?commitPaneMode\(\);[\s\S]*?searchPanel\.open = true;/,
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

describe("Cmd+K pane mode spawn commit (fullstack-a-3)", () => {
  test("1 commits a terminal spawn immediately", () => {
    // `fullstack-a-3`: pressing `1` stages + commits the terminal
    // spawn in the same case so the new tab lands without the user
    // having to press Enter afterwards.
    expect(app).toMatch(
      /case "1": \{[\s\S]*?paneModeStageSpawn\("terminal", resolveSpawnContext\(\)\);[\s\S]*?commitPaneMode\(\);/,
    );
  });

  test("2 commits a browser spawn immediately + primes browserSelection", () => {
    // Browser case must call `revealAndSelect` BEFORE
    // `commitPaneMode()` so the new tab's tree lands already
    // expanded to + selecting the contextual node.
    expect(app).toMatch(
      /case "2": \{[\s\S]*?const ctx = resolveSpawnContext\(\);[\s\S]*?paneModeStageSpawn\("browser", ctx\);[\s\S]*?revealAndSelect\(ctx\.file\);[\s\S]*?revealAndSelect\(ctx\.dir\);[\s\S]*?commitPaneMode\(\);/,
    );
  });

  test("Enter still peeks a staged intent before commit (defensive)", () => {
    // 1/2/3 commit on the keypress now, so the Enter path rarely
    // sees a staged intent in practice. The peek stays as
    // defensive code: if a future affordance stages without
    // committing, Enter still primes `browserSelection`.
    expect(app).toMatch(
      /case "Enter": \{[\s\S]*?const intent = paneMode\.spawnIntent;[\s\S]*?if \(intent && intent\.kind === "browser"\)[\s\S]*?revealAndSelect\(intent\.ctx\.file\);[\s\S]*?revealAndSelect\(intent\.ctx\.dir\);[\s\S]*?commitPaneMode\(\);/,
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
