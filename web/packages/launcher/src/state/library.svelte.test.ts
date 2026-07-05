// State-layer tests against the in-memory mock backend (the default). They
// assert deltas rather than absolute counts, since the mock state is a shared
// module-level singleton mutated across cases.

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  addLocalWorkspace,
  connectDevserver,
  disconnectDevserver,
  library,
  loadLibrary,
  openWorkspaceWindow,
  removeDevserver,
  removeWorkspace,
  saveDevserver,
  stopWatching,
  toggleWorkspace,
} from "./library.svelte";

// Pin the in-memory mock as the backend so these tests drive the registry +
// window feed with no live server, independent of how the app composes its
// default backend. The async-import factory dodges vi.mock's hoist-over-imports
// trap (the factory can't close over a top-level import binding).
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

beforeEach(async () => {
  await loadLibrary();
});

afterEach(() => {
  stopWatching();
  vi.useRealTimers();
});

describe("loadLibrary", () => {
  // The window feed is served live (a real watch socket), so its population is
  // not asserted here: jsdom has no WebSocket, and loadLibrary subscribes
  // best-effort. The feed's grouping/recompose logic is covered by
  // windowLabel.test.ts; its live behaviour by the integration pass.
  it("populates both registries", () => {
    expect(library.workspaces.length).toBeGreaterThanOrEqual(2);
    expect(library.devservers.length).toBeGreaterThanOrEqual(1);
  });

  it("never exposes a devserver token (write-only wire)", () => {
    for (const ds of library.devservers) {
      expect(Object.prototype.hasOwnProperty.call(ds, "token")).toBe(false);
      expect(typeof ds.has_token).toBe("boolean");
    }
  });
});

describe("workspace registry", () => {
  it("adds a local workspace", async () => {
    const before = library.workspaces.length;
    await addLocalWorkspace("/tmp/added-by-test");
    expect(library.workspaces.length).toBe(before + 1);
    expect(library.workspaces.some((w) => w.path === "/tmp/added-by-test")).toBe(true);
  });

  it("toggles a workspace off and on", async () => {
    const ws = library.workspaces[0]!;
    await toggleWorkspace(ws.workspace_id, false);
    expect(library.workspaces.find((w) => w.workspace_id === ws.workspace_id)?.on).toBe(false);
    await toggleWorkspace(ws.workspace_id, true);
    expect(library.workspaces.find((w) => w.workspace_id === ws.workspace_id)?.on).toBe(true);
  });

  it("removes a workspace", async () => {
    await addLocalWorkspace("/tmp/to-remove");
    const target = library.workspaces.find((w) => w.path === "/tmp/to-remove")!;
    const before = library.workspaces.length;
    await removeWorkspace(target.workspace_id);
    expect(library.workspaces.length).toBe(before - 1);
  });

  it("drops a workspace's windows from the feed when it is turned off (no stale state)", async () => {
    // Self-contained against the shared mock: add an on workspace, open a window
    // onto it, then turn it off. The off purges its windows (the backend's
    // discard_workspace_windows, mirrored in the mock) and the watch push
    // replaces library.windows wholesale, so no ghost window record lingers.
    await addLocalWorkspace("/tmp/w6-off");
    const ws = library.workspaces.find((w) => w.path === "/tmp/w6-off")!;
    await openWorkspaceWindow("/tmp/w6-off");
    expect(library.windows.some((w) => w.workspace_path === "/tmp/w6-off")).toBe(true);
    await toggleWorkspace(ws.workspace_id, false);
    expect(library.windows.some((w) => w.workspace_path === "/tmp/w6-off")).toBe(false);
  });
});

describe("devserver registry", () => {
  it("adds a devserver and reports a stored token without echoing it", async () => {
    const before = library.devservers.length;
    await saveDevserver({ host: "box.test", port: 9001, label: "qa", token: "tok_secret" });
    expect(library.devservers.length).toBe(before + 1);
    const added = library.devservers.find((d) => d.host === "box.test" && d.port === 9001)!;
    expect(added.has_token).toBe(true);
    expect(Object.prototype.hasOwnProperty.call(added, "token")).toBe(false);
    expect(added.label).toBe("qa");
  });

  it("edits a devserver, keeping the stored token when none is supplied", async () => {
    await saveDevserver({ host: "edit.test", port: 9002, token: "tok_keep" });
    const ds = library.devservers.find((d) => d.host === "edit.test" && d.port === 9002)!;
    await saveDevserver({ host: "edit.test", port: 9003 }, ds.id);
    const updated = library.devservers.find((d) => d.id === ds.id)!;
    expect(updated.host).toBe("edit.test");
    expect(updated.port).toBe(9003);
    expect(updated.has_token).toBe(true);
  });

  it("edits a devserver and explicitly clears the stored token", async () => {
    await saveDevserver({ host: "clear-token.test", port: 9005, token: "tok_clear" });
    const ds = library.devservers.find((d) => d.host === "clear-token.test" && d.port === 9005)!;
    expect(ds.has_token).toBe(true);

    await saveDevserver({ host: "clear-token.test", port: 9005, clear_token: true }, ds.id);

    const updated = library.devservers.find((d) => d.id === ds.id)!;
    expect(updated.has_token).toBe(false);
  });

  it("removes a devserver", async () => {
    await saveDevserver({ host: "gone.test", port: 9004 });
    const ds = library.devservers.find((d) => d.host === "gone.test" && d.port === 9004)!;
    const before = library.devservers.length;
    await removeDevserver(ds.id);
    expect(library.devservers.length).toBe(before - 1);
  });

  it("connects a devserver and marks its library's windows live", async () => {
    // The seed devserver carries a library id (already connected once); the
    // mock marks that library's windows connected and pushes the feed.
    const ds = library.devservers.find((d) => d.library_id)!;
    library.error = null;
    await connectDevserver(ds.id);
    expect(library.error).toBeNull();
    const remote = library.windows.filter((w) => w.library_id === ds.library_id);
    expect(remote.length).toBeGreaterThan(0);
    expect(remote.every((w) => w.connected)).toBe(true);
  });

  it("disconnects a devserver and removes its transient control row", async () => {
    const ds = library.devservers.find((d) => d.id === "ds-1")!;
    await connectDevserver(ds.id);
    expect(library.windows.some((w) => w.library_id === ds.library_id && w.control)).toBe(true);

    await disconnectDevserver(ds.id);

    expect(library.devservers.find((d) => d.id === ds.id)?.status).toBe("disconnected");
    expect(library.windows.some((w) => w.library_id === ds.library_id && w.control)).toBe(false);
  });
});

describe("resync on regaining visibility/focus", () => {
  it("re-reads both registries on a window focus event", async () => {
    // loadLibrary (beforeEach) installs the visibility/focus listener. A focus
    // re-reads the authoritative registries so state drift while hidden / a
    // feed blip is corrected with no user action.
    const { backend } = await import("../api/backend");
    const ws = vi.spyOn(backend, "listWorkspaces");
    const ds = vi.spyOn(backend, "listDevservers");
    window.dispatchEvent(new Event("focus"));
    await new Promise((r) => setTimeout(r, 0));
    expect(ws).toHaveBeenCalled();
    expect(ds).toHaveBeenCalled();
    ws.mockRestore();
    ds.mockRestore();
  });

  it("polls workspaces while visible and stops after teardown", async () => {
    vi.useFakeTimers();
    stopWatching();
    await loadLibrary();
    const { backend } = await import("../api/backend");
    const ws = vi.spyOn(backend, "listWorkspaces");

    await vi.advanceTimersByTimeAsync(2000);
    expect(ws).toHaveBeenCalled();
    const calls = ws.mock.calls.length;

    stopWatching();
    await vi.advanceTimersByTimeAsync(4000);
    expect(ws.mock.calls.length).toBe(calls);
    ws.mockRestore();
  });
});
