import { describe, expect, test } from "vitest";
import teamWork from "./TeamWork.svelte?raw";
import fileEditor from "./FileEditorTab.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";

// `fullstack-a-86` follow-up to `-a-85`: same-shape swap from
// persistent `ui.status =` to auto-dismissing
// `setTransientStatus()` across 4 more surfaces. Error paths
// + directive surfaces stay persistent.
//
// Phase-13 r2: the Spawn-agents-config copy, the per-terminal
// submit-mode flip, and the agent-event watcher were all deleted
// with the Team Work revamp, so their toast assertions are gone.

describe("fullstack-a-86: confirmed same-shape success swaps", () => {
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

describe("fullstack-a-86: error paths stay persistent", () => {
  test("Team Work copy-failed stays persistent", () => {
    expect(teamWork).toMatch(
      /ui\.status = `copy failed: \$\{\(err as Error\)\.message\}`;/,
    );
  });

  test("FileEditorTab copy-failed stays persistent", () => {
    // Phase-13 slice 3: copy errors now route through the shared
    // copyTextToClipboard helper's onError callback, which still
    // assigns ui.status (= persistent), preserving the contract this
    // test pins. The `msg` shape changed from `(err as Error).message`
    // (raw throw) to the already-resolved string the helper passes
    // back.
    expect(fileEditor).toMatch(
      /onError: \(msg\) => \(ui\.status = `copy failed: \$\{msg\}`\),?/,
    );
  });

  test("Team Work bubble-mode-failed stays persistent", () => {
    expect(teamWork).toMatch(/ui\.status = `bubble mode failed:/);
  });
});

describe("fullstack-a-86: directive + persistent surfaces unchanged", () => {
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
