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

  test("forwards terminal-generated replies unconditionally — reattach-replay gate reverted", () => {
    // The reattach reply-gating (36fcbab5 + 9b44cef2) was REVERTED per @@Alex: it
    // could STALL (the drain depth never returning to 0 under continuous TUI
    // output), so `replayingReattach` stuck true and handleXtermData dropped
    // EVERY live CPR/DA reply — a TUI (claude code / vim) left without its query
    // replies hangs / renders blank. Worse than the leak it prevented. So a
    // terminal-generated reply during a server-write now ALWAYS forwards to its
    // own PTY (sendInput), never the broadcast fan-out, with NO replay-window
    // drop. Accepted tradeoff: an occasional historical CPR reply may echo at the
    // prompt (`…R`/`…c`). Ref dev/terminal-regression/.
    //
    // The gate + ALL its state/logic is gone — no orphans (svelte-check clean).
    expect(terminal).not.toContain("replayingReattach");
    expect(terminal).not.toContain("clearReplayWhenDrained");
    expect(terminal).not.toContain("maybeEndReplayWindow");
    // The depth>0 reply branch forwards straight to its PTY — no conditional drop.
    const handler = terminal.match(
      /function handleXtermData\(data: string\): void \{[\s\S]*?\n  \}/,
    )?.[0];
    expect(handler).toBeTruthy();
    expect(handler).toMatch(
      /if \(ptyOutputWriteDepth > 0\) \{[\s\S]*?sendInput\(data\);\s*return;\s*\}/,
    );
    expect(handler).not.toContain("return;\n      sendInput");
  });
});
