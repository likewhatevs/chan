// Multi-select + bulk-action tests against the in-memory mock. Each case adds
// its own workspaces so it is robust to the shared module-level mock state.

import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  selection,
  isSelected,
  toggleSelected,
  clearSelection,
  bulkSetOn,
  requestBulkDelete,
  cancelBulkDelete,
  confirmBulkDelete,
} from "./selection.svelte";
import { addLocalWorkspace, library, loadLibrary } from "./library.svelte";

// Pin the in-memory mock as the backend so these tests drive the registry with
// no live server, independent of how the app composes its default backend. The
// async-import factory dodges vi.mock's hoist-over-imports trap.
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

beforeEach(async () => {
  clearSelection();
  await loadLibrary();
});

describe("workspace multi-select", () => {
  it("toggles a row in and out of the selection", () => {
    const id = library.workspaces[0]!.workspace_id;
    expect(isSelected(id)).toBe(false);
    toggleSelected(id);
    expect(isSelected(id)).toBe(true);
    expect(selection.selected.length).toBe(1);
    toggleSelected(id);
    expect(isSelected(id)).toBe(false);
    expect(selection.selected.length).toBe(0);
  });

  it("bulk turn off sets every selected workspace off", async () => {
    await addLocalWorkspace("/tmp/sel-a");
    await addLocalWorkspace("/tmp/sel-b");
    const a = library.workspaces.find((w) => w.path === "/tmp/sel-a")!;
    const b = library.workspaces.find((w) => w.path === "/tmp/sel-b")!;
    toggleSelected(a.workspace_id);
    toggleSelected(b.workspace_id);
    await bulkSetOn(false);
    expect(library.workspaces.find((w) => w.workspace_id === a.workspace_id)?.on).toBe(false);
    expect(library.workspaces.find((w) => w.workspace_id === b.workspace_id)?.on).toBe(false);
  });

  it("delete is gated behind a confirm; cancel leaves the row", async () => {
    await addLocalWorkspace("/tmp/sel-keep");
    const id = library.workspaces.find((w) => w.path === "/tmp/sel-keep")!.workspace_id;
    toggleSelected(id);
    requestBulkDelete();
    expect(selection.confirmingDelete).toBe(true);
    cancelBulkDelete();
    expect(selection.confirmingDelete).toBe(false);
    expect(library.workspaces.some((w) => w.workspace_id === id)).toBe(true);
  });

  it("confirmed bulk delete removes the selected workspaces and clears the selection", async () => {
    await addLocalWorkspace("/tmp/sel-del");
    const id = library.workspaces.find((w) => w.path === "/tmp/sel-del")!.workspace_id;
    const before = library.workspaces.length;
    toggleSelected(id);
    requestBulkDelete();
    await confirmBulkDelete();
    expect(library.workspaces.some((w) => w.workspace_id === id)).toBe(false);
    expect(library.workspaces.length).toBe(before - 1);
    expect(selection.selected.length).toBe(0);
    expect(selection.confirmingDelete).toBe(false);
  });
});
