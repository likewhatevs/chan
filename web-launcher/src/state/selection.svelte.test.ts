// Multi-select + bulk-action tests against the in-memory mock. The selection is
// kind-aware (workspaces + devservers); each case adds its own rows so it is
// robust to the shared module-level mock state.

import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  selection,
  isSelected,
  selectedCount,
  toggleSelected,
  clearSelection,
  bulkSetOn,
  requestBulkDelete,
  cancelBulkDelete,
  confirmBulkDelete,
} from "./selection.svelte";
import {
  addLocalWorkspace,
  library,
  loadLibrary,
  saveDevserver,
} from "./library.svelte";

// Pin the in-memory mock as the backend so these tests drive the registries with
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
    const id = library.workspaces.find((w) => w.devserver_id === null)!.workspace_id;
    expect(isSelected("workspace", id)).toBe(false);
    toggleSelected("workspace", id);
    expect(isSelected("workspace", id)).toBe(true);
    expect(selectedCount("workspace")).toBe(1);
    toggleSelected("workspace", id);
    expect(isSelected("workspace", id)).toBe(false);
    expect(selectedCount("workspace")).toBe(0);
  });

  it("bulk turn off sets every selected workspace off", async () => {
    await addLocalWorkspace("/tmp/sel-a");
    await addLocalWorkspace("/tmp/sel-b");
    const a = library.workspaces.find((w) => w.path === "/tmp/sel-a")!;
    const b = library.workspaces.find((w) => w.path === "/tmp/sel-b")!;
    toggleSelected("workspace", a.workspace_id);
    toggleSelected("workspace", b.workspace_id);
    await bulkSetOn("workspace", false);
    expect(library.workspaces.find((w) => w.workspace_id === a.workspace_id)?.on).toBe(false);
    expect(library.workspaces.find((w) => w.workspace_id === b.workspace_id)?.on).toBe(false);
  });

  it("remove is gated behind a confirm; cancel leaves the row", async () => {
    await addLocalWorkspace("/tmp/sel-keep");
    const id = library.workspaces.find((w) => w.path === "/tmp/sel-keep")!.workspace_id;
    toggleSelected("workspace", id);
    requestBulkDelete("workspace");
    expect(selection.confirmingDelete).toBe("workspace");
    cancelBulkDelete();
    expect(selection.confirmingDelete).toBeNull();
    expect(library.workspaces.some((w) => w.workspace_id === id)).toBe(true);
  });

  it("confirmed bulk remove drops the selected workspaces and clears the selection", async () => {
    await addLocalWorkspace("/tmp/sel-del");
    const id = library.workspaces.find((w) => w.path === "/tmp/sel-del")!.workspace_id;
    const before = library.workspaces.length;
    toggleSelected("workspace", id);
    requestBulkDelete("workspace");
    await confirmBulkDelete("workspace");
    expect(library.workspaces.some((w) => w.workspace_id === id)).toBe(false);
    expect(library.workspaces.length).toBe(before - 1);
    expect(selectedCount("workspace")).toBe(0);
    expect(selection.confirmingDelete).toBeNull();
  });
});

describe("devserver multi-select", () => {
  it("keys selection by kind so a workspace and a devserver id never collide", () => {
    const ws = library.workspaces.find((w) => w.devserver_id === null)!.workspace_id;
    const ds = library.devservers[0]!.id;
    toggleSelected("workspace", ws);
    toggleSelected("devserver", ds);
    expect(selectedCount("workspace")).toBe(1);
    expect(selectedCount("devserver")).toBe(1);
    // Clearing one kind leaves the other.
    clearSelection("workspace");
    expect(selectedCount("workspace")).toBe(0);
    expect(selectedCount("devserver")).toBe(1);
  });

  it("bulk turn off disconnects every selected devserver", async () => {
    // The seed devserver is connected; bulk turn off → disconnect.
    const ds = library.devservers.find((d) => d.connected)!;
    toggleSelected("devserver", ds.id);
    await bulkSetOn("devserver", false);
    expect(library.devservers.find((d) => d.id === ds.id)?.connected).toBe(false);
  });

  it("bulk turn on connects every selected devserver", async () => {
    await saveDevserver({ url: "https://bulk-on.example:9200", label: "bulk-on" });
    const ds = library.devservers.find((d) => d.url === "https://bulk-on.example:9200")!;
    expect(ds.connected).toBe(false);
    toggleSelected("devserver", ds.id);
    await bulkSetOn("devserver", true);
    expect(library.devservers.find((d) => d.id === ds.id)?.connected).toBe(true);
  });

  it("confirmed bulk remove drops the selected devserver", async () => {
    await saveDevserver({ url: "https://bulk-rm.example:9300", label: "bulk-rm" });
    const ds = library.devservers.find((d) => d.url === "https://bulk-rm.example:9300")!;
    const before = library.devservers.length;
    toggleSelected("devserver", ds.id);
    requestBulkDelete("devserver");
    expect(selection.confirmingDelete).toBe("devserver");
    await confirmBulkDelete("devserver");
    expect(library.devservers.some((d) => d.id === ds.id)).toBe(false);
    expect(library.devservers.length).toBe(before - 1);
    expect(selectedCount("devserver")).toBe(0);
  });
});
