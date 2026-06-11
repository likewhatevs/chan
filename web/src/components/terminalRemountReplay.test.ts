import { describe, expect, test } from "vitest";
import terminalSource from "./TerminalTab.svelte?raw";

// A TerminalTab remount (pane split, tile swap, cross-pane drag,
// cross-window move) builds a brand-new EMPTY xterm, but the tab object
// can carry the previous xterm's `lastSeq` cursor (directly, or copied
// by cloneTab / the move payload). Attaching with `since=lastSeq` makes
// the server skip the whole replay ring and the terminal shows only
// post-attach output - the "only its last line after a split" bug,
// fixable only by Cmd+R (whose restore paths null lastSeq). start()
// must clear the cursor before the one-and-only connect() so a fresh
// mount always gets the full ring, exactly like a reload.
describe("terminal remount forces full ring replay", () => {
  const src = terminalSource.replace(/\s+/g, " ");

  test("start() clears tab.lastSeq immediately before connect()", () => {
    expect(src).toMatch(/tab\.lastSeq = undefined; void connect\(\);/);
  });

  test("echo dedupe cursor is NOT cleared at mount", () => {
    expect(src).not.toMatch(/tab\.lastAgentEchoSeq = undefined/);
  });

  test("connect() still threads the cursor pair into the ws path", () => {
    // The mechanism stays: reload/move bookkeeping advances lastSeq for
    // session saves, and the ws path consumes whatever the tab holds at
    // attach time (undefined right after mount = replay from seq 0).
    expect(src).toMatch(/lastSeq: tab\.lastSeq,\s*agentEchoSince: tab\.lastAgentEchoSeq,/);
  });
});
