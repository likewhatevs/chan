import { describe, expect, test } from "vitest";
import teamWork from "./TeamWork.svelte?raw";
import fileEditor from "./FileEditorTab.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";

// Success-path status messages use auto-dismissing `setTransientStatus()`
// across surfaces. Error paths and directive surfaces stay persistent.
// The Team Work revamp removed the Spawn-agents-config copy, the
// per-terminal submit-mode flip, and the agent-event watcher, so
// their toast assertions are gone. Wave-1 also removed the bubble-mode
// flip (the rich-prompt / bubble-stub is rebuilt in Wave 2), so its
// "bubble mode failed" toast assertion is gone too.

describe("confirmed same-shape success swaps", () => {
  test("Team Work Copy path uses setTransientStatus", () => {
    expect(teamWork).toMatch(
      /setTransientStatus\("copied metadata dir"\)/,
    );
  });

  test("FileEditorTab `Copied file path` uses setTransientStatus", () => {
    expect(fileEditor).toMatch(
      /setTransientStatus\("Copied file path"\)/,
    );
  });
});

describe("error paths stay persistent", () => {
  test("Team Work copy-failed stays persistent", () => {
    expect(teamWork).toMatch(
      /ui\.status = `copy failed: \$\{\(err as Error\)\.message\}`;/,
    );
  });

  test("FileEditorTab copy-failed stays persistent", () => {
    // Copy errors route through the shared copyTextToClipboard
    // helper's onError callback, which still assigns ui.status
    // (= persistent). The `msg` parameter is the already-resolved
    // error string the helper passes back.
    expect(fileEditor).toMatch(
      /onError: \(msg\) => \(ui\.status = `copy failed: \$\{msg\}`\),?/,
    );
  });
});

describe("directive + persistent surfaces unchanged", () => {
  test("TerminalTab 'PTY did not report CWD' stays persistent (PTY signal)", () => {
    expect(terminal).toMatch(
      /ui\.status = "PTY did not report CWD";/,
    );
  });

  test("FileEditorTab 'Choose the moved file' stays persistent (user directive)", () => {
    expect(fileEditor).toMatch(
      /ui\.status = "Choose the moved file in Files to re-open this tab";/,
    );
  });
});
