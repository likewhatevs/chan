import { describe, expect, test } from "vitest";
import terminalSource from "./TerminalTab.svelte?raw";
import sessionSource from "../terminal/session.ts?raw";

// A TerminalTab (re)mount — pane split, tile swap, cross-pane drag,
// cross-window move, reload — always feeds a brand-new EMPTY xterm, so
// a reattach must always replay the session's full server ring. The
// client once tracked a per-tab byte cursor (`lastSeq`) and sent it as
// the ws `since`; surviving a remount it made the server skip
// everything the DEAD xterm had seen — the "terminal shows only its
// last line after a split" bug. The cursor is gone; these pins keep it
// gone.
describe("terminal reattach always replays the full ring", () => {
  const term = terminalSource.replace(/\s+/g, " ");
  const session = sessionSource.replace(/\s+/g, " ");

  test("the ws path carries no byte cursor — since is the constant 0", () => {
    // Explicit 0 rather than an absent param: Some(0) makes the server
    // report ring-overflow loss via missed_bytes (the "replay missed N
    // bytes" notice); None would silently start at the ring head.
    expect(session).toContain('params.set("since", "0")');
    expect(session).not.toContain("lastSeq");
  });

  test("TerminalTab threads no replay cursor into the attach", () => {
    expect(term).not.toContain("lastSeq");
    expect(term).toMatch(/sessionId: tab\.terminalSessionId,\s*agentEchoSince: tab\.lastAgentEchoSeq,/);
  });

  test("echo dedupe cursor survives the remount", () => {
    // lastAgentEchoSeq is a SEPARATE cursor (Team Work echo dedupe,
    // persisted as `tae`); it is independent of screen content and
    // must NOT be cleared at mount.
    expect(term).not.toMatch(/tab\.lastAgentEchoSeq = undefined/);
  });
});
