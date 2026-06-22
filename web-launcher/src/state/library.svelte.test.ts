// State-layer tests against the in-memory mock backend (the default). They
// assert deltas rather than absolute counts, since the mock state is a shared
// module-level singleton mutated across cases.

import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  addLocalWorkspace,
  library,
  loadLibrary,
  removeDevserver,
  removeWorkspace,
  saveDevserver,
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
});

describe("devserver registry", () => {
  it("adds a devserver and reports a stored token without echoing it", async () => {
    const before = library.devservers.length;
    await saveDevserver({ url: "https://box.test:9001", label: "qa", token: "tok_secret" });
    expect(library.devservers.length).toBe(before + 1);
    const added = library.devservers.find((d) => d.url === "https://box.test:9001")!;
    expect(added.has_token).toBe(true);
    expect(Object.prototype.hasOwnProperty.call(added, "token")).toBe(false);
    expect(added.label).toBe("qa");
  });

  it("edits a devserver, keeping the stored token when none is supplied", async () => {
    await saveDevserver({ url: "https://edit.test:9002", token: "tok_keep" });
    const ds = library.devservers.find((d) => d.url === "https://edit.test:9002")!;
    await saveDevserver({ url: "https://edit.test:9003" }, ds.id);
    const updated = library.devservers.find((d) => d.id === ds.id)!;
    expect(updated.url).toBe("https://edit.test:9003");
    expect(updated.has_token).toBe(true);
  });

  it("removes a devserver", async () => {
    await saveDevserver({ url: "https://gone.test:9004" });
    const ds = library.devservers.find((d) => d.url === "https://gone.test:9004")!;
    const before = library.devservers.length;
    await removeDevserver(ds.id);
    expect(library.devservers.length).toBe(before - 1);
  });
});
