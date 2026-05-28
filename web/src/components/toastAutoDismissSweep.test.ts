import { describe, expect, test } from "vitest";
import richPrompt from "./TerminalRichPrompt.svelte?raw";
import fileEditor from "./FileEditorTab.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";

// `fullstack-a-86` follow-up to `-a-85`: same-shape swap from
// persistent `ui.status =` to auto-dismissing
// `setTransientStatus()` across 4 more surfaces. Error paths
// + directive surfaces stay persistent.

describe("fullstack-a-86: confirmed same-shape success swaps", () => {
  test("TerminalRichPrompt metadata copy uses setTransientStatus", () => {
    expect(richPrompt).toMatch(
      /setTransientStatus\("copied metadata dir"\)/,
    );
  });

  test("TerminalRichPrompt Spawn agents config copy uses setTransientStatus", () => {
    expect(richPrompt).toMatch(
      /setTransientStatus\("copied spawn agents config"\)/,
    );
  });

  test("FileEditorTab `Copied file path` uses setTransientStatus", () => {
    expect(fileEditor).toMatch(
      /setTransientStatus\("Copied file path"\)/,
    );
  });
});

describe("fullstack-a-86: debatable info swaps (watcher detached on reload)", () => {
  test("TerminalTab watcher-detached toast auto-dismisses", () => {
    expect(terminal).toMatch(
      /setTransientStatus\("watcher detached on reload"\)/,
    );
  });
});

describe("fullstack-a-86: error paths stay persistent", () => {
  test("TerminalRichPrompt copy-failed stays persistent", () => {
    expect(richPrompt).toMatch(
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

  test("TerminalRichPrompt submit-mode-flip-failed stays persistent", () => {
    expect(richPrompt).toMatch(
      /ui\.status = `submit-mode flip failed:/,
    );
  });

  test("TerminalRichPrompt bubble-mode-failed stays persistent", () => {
    expect(richPrompt).toMatch(/ui\.status = `bubble mode failed:/);
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
