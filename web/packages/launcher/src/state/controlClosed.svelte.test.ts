// The devserver control-closed dispatch is flash-only: the `devserver-control-
// closed` event flashes the control row's eye for attention (no survey modal).
// These tests cover the id parser (both wire shapes) and that the event flags
// the devserver's control library for attention. Run against the in-memory mock
// (seeded devserver `ds-1`, "prod", connected, library DS_LIBRARY_ID).

import { describe, it, expect, beforeEach, vi } from "vitest";
import { controlClosedId, onControlClosedEvent } from "./controlClosed.svelte";
import { library, loadLibrary } from "./library.svelte";
import { hasControlAttention, clearAllControlAttention } from "./controlAttention.svelte";

vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

beforeEach(async () => {
  clearAllControlAttention();
  // Seeds the devserver registry incl. ds-1, so markControlAttention can resolve
  // the devserver id to its library id.
  await loadLibrary();
});

describe("controlClosedId", () => {
  it("parses a bare string id", () => {
    expect(controlClosedId("ds-1")).toBe("ds-1");
  });

  it("parses an { id } object", () => {
    expect(controlClosedId({ id: "ds-2" })).toBe("ds-2");
  });

  it("returns null for an unrecognized payload", () => {
    expect(controlClosedId(null)).toBeNull();
    expect(controlClosedId(42)).toBeNull();
    expect(controlClosedId("")).toBeNull();
    expect(controlClosedId({})).toBeNull();
  });
});

describe("onControlClosedEvent (flash-only)", () => {
  it("flashes the devserver's control library for attention", () => {
    const libId = library.devservers.find((d) => d.id === "ds-1")!.library_id!;
    expect(hasControlAttention(libId)).toBe(false);
    onControlClosedEvent("ds-1");
    expect(hasControlAttention(libId)).toBe(true);
  });

  it("ignores an unrecognized payload", () => {
    const libId = library.devservers.find((d) => d.id === "ds-1")!.library_id!;
    onControlClosedEvent(null);
    expect(hasControlAttention(libId)).toBe(false);
  });
});
