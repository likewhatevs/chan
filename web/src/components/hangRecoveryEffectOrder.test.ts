import { describe, expect, test } from "vitest";
import fileEditor from "./FileEditorTab.svelte?raw";

// `fullstack-a-83` HIGH: 3rd-round follow-up to `-a-82`.
// @@WebtestA's `206c010` walk: path-keying works (buffer key
// confirmed `chan:editor-buffer:CLAUDE.md`) but the banner
// STILL doesn't surface empirically on divergent reload.
//
// Root cause: when `tab.saved` arrives + `tab.content` also
// equals `tab.saved` (both just loaded from disk), the
// persistence effect's clean-state branch fires
// `clearEditorBuffer(tab.path)` — which clobbers the buffer
// the mount effect was about to surface (or just surfaced).
// Depending on microtask order, the banner either flashes
// once and disappears OR never renders at all.
//
// Fix: in the clean-state branch, skip the
// `clearEditorBuffer` + `cancelPendingBufferWrite` calls
// when `recoveredBuffer !== null` (banner is up; let the
// user click Restore / Discard before clearing).

describe("fullstack-a-83: persistence-effect clean-state guard", () => {
  test("clean-state branch skips clearing when recoveredBuffer is set", () => {
    expect(fileEditor).toMatch(
      /if \(content === saved\) \{[\s\S]*?if \(recoveredBuffer !== null\) \{[\s\S]*?return;[\s\S]*?\}[\s\S]*?cancelPendingBufferWrite\(tab\.path\);[\s\S]*?clearEditorBuffer\(tab\.path\);/,
    );
  });

  test("rationale comment cites the effect-ordering race", () => {
    expect(fileEditor).toMatch(/effect-ordering/i);
    expect(fileEditor).toMatch(
      /clearEditorBuffer.*can wipe[\s\S]*?between the mount effect's read and the user's/i,
    );
  });

  test("clean-state branch still clears when banner is dismissed (recoveredBuffer null)", () => {
    // The else-equivalent (banner not up) path is the
    // sequential `cancelPendingBufferWrite + clearEditorBuffer`
    // pair — that's the normal pre-`-a-83` behavior, gated by
    // the guard.
    expect(fileEditor).toMatch(
      /Banner is up[\s\S]*?leave the buffer in place[\s\S]*?Restore/i,
    );
  });
});

describe("fullstack-a-83: discardBuffer follow-up fix", () => {
  test("discardBuffer clears by tab.path (not the stale tab.id)", () => {
    // Pre-`-a-83` `discardBuffer` still called
    // `clearEditorBuffer(tab.id)` — a stale relic from
    // before `-a-82`'s path-keying re-key. It silently
    // no-op'd because `tab.id` changes on every reload.
    expect(fileEditor).toMatch(
      /function discardBuffer\(\): void \{[\s\S]*?clearEditorBuffer\(tab\.path\);[\s\S]*?recoveredBuffer = null;/,
    );
  });

  test("pre-fix tab.id discard call is gone", () => {
    expect(fileEditor).not.toMatch(
      /function discardBuffer\(\): void \{[\s\S]*?clearEditorBuffer\(tab\.id\)/,
    );
  });
});
