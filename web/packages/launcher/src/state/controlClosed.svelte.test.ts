// State test: the devserver control-terminal-closed survey.
//
// The exit watcher's `devserver-control-closed` event opens a single survey;
// Re-run reconnects (disconnect+connect), Abandon disconnects, Edit disconnects
// and opens the devserver edit form. Exercises the real Svelte 5 runtime against
// the in-memory mock backend (seeded devserver `ds-1`, "prod", connected).

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { flushSync } from "svelte";
import { library, loadLibrary } from "./library.svelte";
import { dialog, closeDialog } from "./dialog.svelte";
import {
  controlClosed,
  controlClosedId,
  onControlClosedEvent,
  handleControlClosed,
  rerunControlClosed,
  editControlClosed,
  abandonControlClosed,
  dismissControlClosed,
} from "./controlClosed.svelte";

vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

// A macrotask hop settles every awaited promise in an action's chain.
function settle(): Promise<void> {
  return new Promise((r) => setTimeout(r, 0));
}

function ds1Connected(): boolean {
  return library.devservers.find((d) => d.id === "ds-1")!.status === "connected";
}

beforeEach(async () => {
  const { resetMockRemoteWorkspaces } = await import("../api/mock");
  resetMockRemoteWorkspaces();
  // Restore the seeded connected state: a prior test in this file may have
  // disconnected ds-1, and the workspace reset above doesn't touch the
  // devserver connection flag. connectDevserver is idempotent.
  const { backend } = await import("../api/backend");
  await backend.connectDevserver("ds-1");
  library.error = null;
  dismissControlClosed();
  closeDialog();
  await loadLibrary();
});

afterEach(() => {
  dismissControlClosed();
  closeDialog();
});

describe("controlClosedId — payload shape", () => {
  it("accepts the bare string id the desktop emits", () => {
    expect(controlClosedId("ds-1")).toBe("ds-1");
  });
  it("also tolerates an { id } object (old native shape)", () => {
    expect(controlClosedId({ id: "ds-1" })).toBe("ds-1");
  });
  it("returns null for an empty / unrecognized payload", () => {
    expect(controlClosedId("")).toBeNull();
    expect(controlClosedId({})).toBeNull();
    expect(controlClosedId(null)).toBeNull();
    expect(controlClosedId(42)).toBeNull();
  });
});

describe("handleControlClosed — open + dedup", () => {
  it("opens the survey titled with the devserver's name", () => {
    expect(controlClosed.open).toBe(false);
    handleControlClosed("ds-1");
    expect(controlClosed.open).toBe(true);
    expect(controlClosed.id).toBe("ds-1");
    expect(controlClosed.name).toBe("prod");
  });

  it("falls back to a generic name when the registry row is gone", () => {
    handleControlClosed("missing");
    expect(controlClosed.name).toBe("The devserver");
  });

  it("ignores a second event while a survey is open (single modal)", () => {
    handleControlClosed("ds-1");
    handleControlClosed("ds-other");
    expect(controlClosed.id).toBe("ds-1");
  });

  it("onControlClosedEvent extracts the id and opens the survey", () => {
    onControlClosedEvent("ds-1");
    expect(controlClosed.open).toBe(true);
    expect(controlClosed.id).toBe("ds-1");
  });

  it("onControlClosedEvent ignores an unrecognized payload", () => {
    onControlClosedEvent(null);
    expect(controlClosed.open).toBe(false);
  });
});

describe("survey actions", () => {
  it("Re-run reconnects (disconnect+connect) and closes the survey", async () => {
    handleControlClosed("ds-1");
    await rerunControlClosed();
    await settle();
    flushSync();
    expect(ds1Connected()).toBe(true);
    expect(controlClosed.open).toBe(false);
    expect(library.error).toBeNull();
  });

  it("Abandon disconnects the devserver and closes the survey", async () => {
    handleControlClosed("ds-1");
    await abandonControlClosed();
    await settle();
    flushSync();
    expect(ds1Connected()).toBe(false);
    expect(controlClosed.open).toBe(false);
    expect(library.error).toBeNull();
  });

  it("Edit disconnects then opens the devserver edit form, survey closed", async () => {
    handleControlClosed("ds-1");
    await editControlClosed();
    await settle();
    flushSync();
    expect(ds1Connected()).toBe(false);
    expect(controlClosed.open).toBe(false);
    expect(dialog.open).toBe(true);
    expect(dialog.choice).toBe("devserver");
    expect(dialog.editing?.id).toBe("ds-1");
  });

  it("Dismiss closes the survey without changing the connection", async () => {
    handleControlClosed("ds-1");
    const before = ds1Connected();
    dismissControlClosed();
    await settle();
    expect(controlClosed.open).toBe(false);
    expect(ds1Connected()).toBe(before);
  });

  it("Abandon surfaces a disconnect failure in the banner", async () => {
    const { backend } = await import("../api/backend");
    const spy = vi
      .spyOn(backend, "disconnectDevserver")
      .mockRejectedValueOnce(new Error("NO_DESKTOP"));
    handleControlClosed("ds-1");
    await abandonControlClosed();
    await settle();
    flushSync();
    expect(controlClosed.open).toBe(false);
    expect(library.error).toBe("NO_DESKTOP");
    spy.mockRestore();
  });
});
