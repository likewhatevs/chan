import { describe, expect, test } from "vitest";
import fileEditor from "./FileEditorTab.svelte?raw";

// `fullstack-a-82` HIGH — banner failed empirically because the
// pre-`-a-82` buffer key was `chan:editor-buffer:<tab.id>` and
// tab ids are module-counter-generated from a `nextId` that
// resets on every page load. The persisted key became a dead
// reference after reload + the new tab's mount-time read
// returned null + no banner.
//
// Fix: re-key the buffer on `tab.path` (stable across reloads).
// Plus guard the persistence effect from running while
// `tab.saved` is still undefined (initial mount before the
// file fetch completes) — otherwise an empty initial-content
// write could land in localStorage + clobber the just-restored
// buffer.

describe("fullstack-a-82: FileEditorTab uses tab.path as buffer key", () => {
  test("mount-time divergence check passes tab.path (not tab.id)", () => {
    expect(fileEditor).toMatch(
      /recoveredBuffer = divergentBufferOrNull\(tab\.path, tab\.path, disk\)/,
    );
  });

  test("graceful unmount cancels pending write by tab.path", () => {
    expect(fileEditor).toMatch(/cancelPendingBufferWrite\(tab\.path\)/);
  });

  test("persistence effect queues writes by tab.path", () => {
    expect(fileEditor).toMatch(
      /queueBufferWrite\(tab\.path, content, tab\.path\)/,
    );
  });

  test("clean-state branch clears editor buffer by tab.path", () => {
    expect(fileEditor).toMatch(/clearEditorBuffer\(tab\.path\)/);
  });
});

describe("fullstack-a-82: persistence effect waits for disk content load", () => {
  test("effect short-circuits when tab.saved is undefined (disk fetch in flight)", () => {
    // Pre-`-a-82` the effect treated `saved === undefined` as
    // "diverges from empty content" and queued a `""` write
    // that could clobber the restored buffer after the 500ms
    // debounce.
    expect(fileEditor).toMatch(/if \(saved === undefined\) return;/);
  });

  test("comment documents the disk-load race", () => {
    expect(fileEditor).toMatch(
      /disk content hasn't finished loading[\s\S]*?clobber the freshly-restored buffer/i,
    );
  });
});

describe("fullstack-a-82: rationale comments", () => {
  test("mount-effect comment cites the tab.id regeneration bug", () => {
    expect(fileEditor).toMatch(
      /Tab ids are module-counter-generated[\s\S]*?reset on every page load/i,
    );
  });
});
