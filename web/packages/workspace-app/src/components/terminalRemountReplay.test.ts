import { describe, expect, test } from "vitest";
import terminalSource from "./TerminalTab.svelte?raw";
import sessionSource from "../terminal/session.ts?raw";

// A TerminalTab (re)mount -- pane split, tile swap, cross-pane drag,
// cross-window move, reload -- feeds a brand-new EMPTY xterm; a reattach
// RESUMES from a cached scrollback snapshot instead of always
// re-replaying the full server ring -- but only in the SAFE form that does NOT
// reintroduce the old "terminal shows only its last line after a split" bug:
// the byte cursor is never a bare per-tab field. It rides ONLY with a
// localStorage snapshot whose geometry + session generation still match, the
// snapshot CONTENT is primed alongside the cursor, and any mismatch (generation
// or missed bytes) falls back to the full replay. These pins keep that shape.
describe("terminal reattach resumes safely or full-replays", () => {
  const term = terminalSource.replace(/\s+/g, " ");
  const session = sessionSource.replace(/\s+/g, " ");

  test("the ws path defaults to since=0 and pairs a real cursor with a generation", () => {
    // Default is still an explicit 0 (Some(0) makes the server report
    // ring-overflow loss via missed_bytes); a real cursor comes from opts.since
    // and the generation rides only when present so the server can gate it.
    expect(session).toContain(
      'params.set("since", String(Math.max(0, Math.floor(opts.since ?? 0))))',
    );
    expect(session).toContain("if (opts.generation != null)");
  });

  test("TerminalTab sources its resume cursor from the snapshot cache, geometry-gated", () => {
    // The cursor is NOT a bare tab field; it is read from the localStorage
    // snapshot only when the cached cols/rows still match the live xterm.
    expect(term).toContain("readTerminalSnapshot");
    expect(term).toMatch(
      /cached\.cols === term\.cols && cached\.rows === term\.rows/,
    );
  });

  test("a cached snapshot primes only on a generation + missed match, else full replay", () => {
    // The snapshot is written into the fresh xterm ONLY when the prelude
    // confirms the same generation and no missed bytes; otherwise it is dropped
    // and the server's full replay repaints from scratch (the fallback).
    expect(term).toMatch(
      /frame\.generation === pendingSnapshot\.generation &&\s*\(frame\.missed_bytes \?\? 0\) === 0/,
    );
    expect(term).toContain("pendingSnapshot = null");
  });

  test("echo dedupe cursor survives the remount", () => {
    // lastAgentEchoSeq is a SEPARATE cursor (Team Work echo dedupe, persisted
    // as `tae`); independent of screen content, never cleared at mount.
    expect(term).not.toMatch(/tab\.lastAgentEchoSeq = undefined/);
  });
});
