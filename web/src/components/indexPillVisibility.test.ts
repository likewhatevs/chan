// @vitest-environment jsdom

// Phase-11 Slice G: the index/build progress pill stays visible while
// the indexer is working and CLEARS the moment it reports idle. This is
// the UI half of the bug-9 fix (Slice D made the server-side status
// actually reach idle through the embed phase + on cancel/reset; this
// test locks the AppStatusBar visibility rule so a future status-flow
// change can't silently re-strand the pill).
//
// AppStatusBar derives `indexVisible` from the shared `indexStatus`
// store; we drive the store directly and assert the same predicate, plus
// pin the source so the derivation and the template branch stay in
// lockstep.

import { afterEach, describe, expect, test } from "vitest";
import statusBar from "./AppStatusBar.svelte?raw";
import { indexStatus } from "../state/store.svelte";
import type { IndexStatus } from "../api/types";

/// The exact predicate AppStatusBar uses for `indexVisible`. Kept here
/// as the spec; the source-pinning test below guards that the component
/// still computes it this way.
function indexVisible(value: IndexStatus | null): boolean {
  return value !== null && value.state !== "idle";
}

afterEach(() => {
  indexStatus.value = null;
});

describe("Slice G: index progress pill visibility", () => {
  test("hidden before the first poll reply (null)", () => {
    indexStatus.value = null;
    expect(indexVisible(indexStatus.value)).toBe(false);
  });

  test("visible while building (counter + file animate through embed)", () => {
    indexStatus.value = {
      state: "building",
      current: 42,
      total: 100,
      file: "embedding",
    } as IndexStatus;
    expect(indexVisible(indexStatus.value)).toBe(true);
  });

  test("visible while reindexing a single file", () => {
    indexStatus.value = { state: "reindexing", file: "notes/x.md" } as IndexStatus;
    expect(indexVisible(indexStatus.value)).toBe(true);
  });

  test("visible on error", () => {
    indexStatus.value = { state: "error", message: "boom" } as IndexStatus;
    expect(indexVisible(indexStatus.value)).toBe(true);
  });

  test("CLEARS the moment the indexer reports idle (bug 9)", () => {
    indexStatus.value = {
      state: "building",
      current: 99,
      total: 100,
      file: "embedding",
    } as IndexStatus;
    expect(indexVisible(indexStatus.value)).toBe(true);

    // Slice D guarantees the status reaches idle; the pill must hide.
    indexStatus.value = { state: "idle" } as IndexStatus;
    expect(indexVisible(indexStatus.value)).toBe(false);
  });
});

describe("Slice G: AppStatusBar source keeps the idle-hide rule", () => {
  test("indexVisible derivation hides on idle and null", () => {
    expect(statusBar).toMatch(
      /indexStatus\.value !== null && indexStatus\.value\.state !== "idle"/,
    );
  });

  test("building branch surfaces the animated counter so the embed phase isn't a frozen pill", () => {
    expect(statusBar).toMatch(/s\.state === "building"/);
    expect(statusBar).toMatch(/\{s\.current\}\/\{s\.total\}/);
  });
});
