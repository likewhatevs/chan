// Unit tests for the in-flight pending store: begin → reconcile clears once the
// row reaches its target (or the row is gone, or it times out); clear stops the
// spinner (the reject path); and the localStorage round-trip (persist + hydrate)
// survives a reload. A fake localStorage is stubbed so the persistence path is
// deterministic regardless of the jsdom/node localStorage quirk.

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  pending,
  isPending,
  beginPending,
  clearPending,
  clearAllPending,
  hydratePending,
  reconcile,
  wsKey,
  servedKey,
  dsKey,
} from "./pending.svelte";

const STORAGE_KEY = "chan-launcher-pending";

// A minimal in-memory Storage so persist()/load() are deterministic.
function fakeStorage(): Storage {
  const m = new Map<string, string>();
  return {
    getItem: (k) => m.get(k) ?? null,
    setItem: (k, v) => {
      m.set(k, String(v));
    },
    removeItem: (k) => {
      m.delete(k);
    },
    clear: () => m.clear(),
    key: (i) => [...m.keys()][i] ?? null,
    get length() {
      return m.size;
    },
  } as Storage;
}

beforeEach(() => {
  vi.stubGlobal("localStorage", fakeStorage());
  clearAllPending();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("pending store", () => {
  it("begins a marker and reports it pending", () => {
    const k = wsKey("a");
    expect(isPending(k)).toBe(false);
    beginPending(k, "on");
    expect(isPending(k)).toBe(true);
    expect(pending.markers[k]?.target).toBe("on");
  });

  it("keys served + devserver markers distinctly", () => {
    beginPending(servedKey("ds-1", "w/api"), "off");
    beginPending(dsKey("ds-1"), "connected");
    expect(isPending(servedKey("ds-1", "w/api"))).toBe(true);
    expect(isPending(dsKey("ds-1"))).toBe(true);
    // A different devserver serving the same prefix is a different marker.
    expect(isPending(servedKey("ds-2", "w/api"))).toBe(false);
  });

  it("reconcile clears a marker once its row reaches the target, keeps it otherwise", () => {
    beginPending(wsKey("a"), "on");
    beginPending(wsKey("b"), "on");
    // a reached "on" → cleared; b still "off" → kept.
    reconcile({ [wsKey("a")]: "on", [wsKey("b")]: "off" });
    expect(isPending(wsKey("a"))).toBe(false);
    expect(isPending(wsKey("b"))).toBe(true);
  });

  it("reconcile clears a marker whose row is gone (no current state)", () => {
    beginPending(dsKey("x"), "connected");
    reconcile({}); // x no longer present
    expect(isPending(dsKey("x"))).toBe(false);
  });

  it("treats a timed-out marker as not pending and reconcile drops it", () => {
    const k = wsKey("c");
    beginPending(k, "on");
    // Backdate the marker well past the timeout.
    pending.markers[k]!.ts = Date.now() - 60_000;
    expect(isPending(k)).toBe(false);
    // Even though the row has NOT reached the target, the timeout clears it.
    reconcile({ [k]: "off" });
    expect(k in pending.markers).toBe(false);
  });

  it("clearPending stops the spinner (the reject path)", () => {
    const k = dsKey("r");
    beginPending(k, "connected");
    expect(isPending(k)).toBe(true);
    clearPending(k);
    expect(isPending(k)).toBe(false);
  });

  it("persists to localStorage and hydrates back (survives reload)", () => {
    const k = dsKey("p");
    beginPending(k, "connected");
    // Persisted under the storage key.
    const raw = localStorage.getItem(STORAGE_KEY);
    expect(raw).toBeTruthy();
    expect(JSON.parse(raw!)[k].target).toBe("connected");

    // Simulate a reload: a fresh in-memory store loses the runtime state, but
    // hydrate() re-reads the persisted markers.
    clearAllPending();
    expect(isPending(k)).toBe(false);
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ [k]: { target: "connected", ts: Date.now() } }),
    );
    hydratePending();
    expect(isPending(k)).toBe(true);
  });

  it("drops malformed persisted entries on hydrate", () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ "ws:ok": { target: "on", ts: Date.now() }, "ws:bad": { target: "nope" } }),
    );
    hydratePending();
    expect(isPending(wsKey("ok"))).toBe(true);
    expect(isPending(wsKey("bad"))).toBe(false);
  });
});
