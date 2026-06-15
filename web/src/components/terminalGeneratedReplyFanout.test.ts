import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";

describe("TerminalTab generated reply routing", () => {
  test("installs xterm OSC color report guards", () => {
    expect(terminal).toContain(
      'import { installTerminalReportGuards } from "../terminal/xtermReports";',
    );
    // The keyboard-protocol state lives on the tab (survives a remount on
    // reattach); start() resets it only on a fresh spawn, then installs
    // the report guards.
    expect(terminal).toMatch(
      /term = new Terminal\([\s\S]*?\);[\s\S]*?const keyboardProtocol = ensureTerminalKeyboardProtocol\([\s\S]*?\);\s*installTerminalReportGuards\(term\);/,
    );
  });

  test("installs keyboard protocol tracking before xterm input handlers", () => {
    expect(terminal).toMatch(
      /installKeyboardProtocolHandlers\(term, keyboardProtocol, sendInput\);[\s\S]*?term\.attachCustomKeyEventHandler\(handleTerminalKeyEvent\);/,
    );
  });

  test("PTY output uses a tracked xterm write", () => {
    expect(terminal).toMatch(
      /function writePtyOutput\(bytes: Uint8Array\): void \{[\s\S]*?ptyOutputWriteDepth \+= 1;[\s\S]*?term\.write\(bytes, \(\) => \{[\s\S]*?ptyOutputWriteDepth = Math\.max\(0, ptyOutputWriteDepth - 1\);/,
    );
    expect(terminal).toMatch(/const bytes = new Uint8Array\(event\.data\);[\s\S]*?writePtyOutput\(bytes\);/);
  });

  test("xterm-generated replies bypass broadcast fan-out", () => {
    expect(terminal).toContain("term.onData(handleXtermData);");
    expect(terminal).toMatch(
      /function handleXtermData\(data: string\): void \{[\s\S]*?if \(ptyOutputWriteDepth > 0\) \{[\s\S]*?sendInput\(data\);[\s\S]*?return;[\s\S]*?\}[\s\S]*?sendUserInput\(data\);/,
    );
  });

  test("drops xterm replies during the reattach replay window (CPR-leak fix)", () => {
    // A replayed historical query (e.g. ESC[6n) must NOT have its reply sent to
    // the PTY — that lands it at the prompt + echoes. The gate is scoped to the
    // replay window only (set on reattach, cleared on `ready`), so LIVE replies
    // still forward.
    expect(terminal).toContain("let replayingReattach = false;");
    expect(terminal).toContain("replayingReattach = reattaching;");
    // Cleared on the `ready` frame (the server sends it AFTER the replay ring).
    expect(terminal).toMatch(
      /frame\.type === "ready"[\s\S]*?replayingReattach = false;/,
    );
    // The drop: during the replay window, return without sendInput.
    expect(terminal).toMatch(
      /if \(ptyOutputWriteDepth > 0\) \{[\s\S]*?if \(replayingReattach\) return;[\s\S]*?sendInput\(data\);/,
    );
  });
});
