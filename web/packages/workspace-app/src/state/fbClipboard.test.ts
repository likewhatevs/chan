// @vitest-environment jsdom
//
// FB2 (File Browser clipboard): cmd/ctrl+C/X/V over the per-instance
// multi-selection. The store owns the clipboard state + the paste, which
// routes through POST /api/fs/transfer (op=copy for a copy, op=move for a
// cut). These tests pin the transitions + the wire op so the clipboard
// can't silently regress.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

// Mock the transfer endpoint before the store module evaluates so the
// import binding the store reads is the mock.
const fsTransfer =
  vi.fn<
    (
      op: "move" | "copy",
      sources: string[],
      destDir: string,
    ) => Promise<{ moved: Array<{ from: string; to: string }>; skipped: string[]; conflicts: string[] }>
  >();

vi.mock("../api/client", () => ({
  api: {
    fsTransfer: (op: "move" | "copy", sources: string[], destDir: string) =>
      fsTransfer(op, sources, destDir),
  },
}));

let store: typeof import("./store.svelte");

beforeEach(async () => {
  vi.resetAllMocks();
  store = await import("./store.svelte");
  store.fbClipboardClear();
});

afterEach(() => {
  store.fbClipboardClear();
});

describe("FB clipboard (FB2)", () => {
  test("copy captures a snapshot of the selection as mode=copy", () => {
    store.fbClipboardSet("copy", ["notes/a.md", "notes/b.md"]);
    expect(store.fbClipboard.mode).toBe("copy");
    expect(store.fbClipboard.paths).toEqual(["notes/a.md", "notes/b.md"]);
  });

  test("cut captures the selection as mode=cut", () => {
    store.fbClipboardSet("cut", ["notes/a.md"]);
    expect(store.fbClipboard.mode).toBe("cut");
    expect(store.fbClipboard.paths).toEqual(["notes/a.md"]);
  });

  test("set is a no-op for an empty selection", () => {
    store.fbClipboardSet("copy", []);
    expect(store.fbClipboard.mode).toBeNull();
    expect(store.fbClipboard.paths).toEqual([]);
  });

  test("the snapshot is independent of later selection mutation", () => {
    const sel = ["notes/a.md", "notes/b.md"];
    store.fbClipboardSet("copy", sel);
    sel.push("notes/c.md"); // mutate the caller's array after capture
    expect(store.fbClipboard.paths).toEqual(["notes/a.md", "notes/b.md"]);
  });

  test("paste of a copy calls fsTransfer with op=copy and keeps the clipboard", async () => {
    fsTransfer.mockResolvedValue({
      moved: [{ from: "notes/a.md", to: "archive/a.md" }],
      skipped: [],
      conflicts: [],
    });
    store.fbClipboardSet("copy", ["notes/a.md"]);
    const landed = await store.fbClipboardPaste("archive");
    expect(fsTransfer).toHaveBeenCalledWith("copy", ["notes/a.md"], "archive");
    expect(landed).toEqual(["archive/a.md"]);
    // A copy clipboard persists so it can be pasted again.
    expect(store.fbClipboard.mode).toBe("copy");
  });

  test("paste of a cut calls fsTransfer with op=move and clears the clipboard", async () => {
    fsTransfer.mockResolvedValue({
      moved: [{ from: "notes/a.md", to: "archive/a.md" }],
      skipped: [],
      conflicts: [],
    });
    store.fbClipboardSet("cut", ["notes/a.md"]);
    const landed = await store.fbClipboardPaste("archive");
    expect(fsTransfer).toHaveBeenCalledWith("move", ["notes/a.md"], "archive");
    expect(landed).toEqual(["archive/a.md"]);
    // A cut is one-shot: the clipboard empties so the source can't be
    // moved a second time.
    expect(store.fbClipboard.mode).toBeNull();
    expect(store.fbClipboard.paths).toEqual([]);
  });

  test("paste with an empty clipboard is a no-op (no transfer call)", async () => {
    const landed = await store.fbClipboardPaste("archive");
    expect(fsTransfer).not.toHaveBeenCalled();
    expect(landed).toEqual([]);
  });

  test("clear empties the clipboard", () => {
    store.fbClipboardSet("cut", ["notes/a.md"]);
    store.fbClipboardClear();
    expect(store.fbClipboard.mode).toBeNull();
    expect(store.fbClipboard.paths).toEqual([]);
  });

  test("a failed paste keeps the clipboard (so the user can retry)", async () => {
    fsTransfer.mockRejectedValue(new Error("boom"));
    store.fbClipboardSet("cut", ["notes/a.md"]);
    const landed = await store.fbClipboardPaste("archive");
    expect(landed).toEqual([]);
    // Not cleared on failure: a cut that errored is still pending.
    expect(store.fbClipboard.mode).toBe("cut");
  });
});
