import { describe, expect, test } from "vitest";
import fileEditor from "./FileEditorTab.svelte?raw";
import terminal from "./TerminalTab.svelte?raw";
import editorCommands from "../state/commands/editor.ts?raw";
import store from "../state/store.svelte.ts?raw";

// Success-path status messages use auto-dismissing `setTransientStatus()`
// across surfaces. Error paths and directive surfaces stay persistent.

describe("confirmed same-shape success swaps", () => {
  test("editor copy-path command reaches transient notify path", () => {
    expect(editorCommands).toMatch(
      /id: "app\.editor\.copyPath"[\s\S]{1,500}onSuccess: \(\) => notify\("Copied file path"\)/,
    );
    expect(store).toMatch(
      /setNotifyHandler\(\(msg\) => \{[\s\S]{1,120}setTransientStatus\(msg\);/,
    );
  });
});

describe("error paths stay persistent", () => {
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
