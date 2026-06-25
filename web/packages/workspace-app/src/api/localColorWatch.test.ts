// @vitest-environment jsdom

// Dedicated per-library focus-colour watch. Pins that it opens a
// WebSocket to /api/library/local-color/watch, delivers `{ color }` frames (hex
// or null) to the callback, drops malformed frames, and the disposer closes the
// socket + defuses the reconnect. jsdom has no WebSocket, so a controllable fake
// stands in (mirrors fbWatch.test.ts).

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { openLocalColorWatch } from "./transport";

class FakeWebSocket {
  static OPEN = 1;
  static instances: FakeWebSocket[] = [];

  readyState = 0; // CONNECTING
  onopen: (() => void) | null = null;
  onmessage: ((m: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(public url: string) {
    FakeWebSocket.instances.push(this);
  }
  open(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.();
  }
  message(data: string): void {
    this.onmessage?.({ data });
  }
  close(): void {
    this.readyState = 3; // CLOSED
  }
}

function latest(): FakeWebSocket {
  const ws = FakeWebSocket.instances.at(-1);
  if (!ws) throw new Error("no socket opened");
  return ws;
}

beforeEach(() => {
  FakeWebSocket.instances = [];
  vi.stubGlobal("WebSocket", FakeWebSocket as unknown as typeof WebSocket);
  window.history.replaceState(null, "", "/");
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("openLocalColorWatch", () => {
  test("opens a WS to the local-color watch path", () => {
    const dispose = openLocalColorWatch(() => {});
    expect(latest().url).toContain("/api/library/local-color/watch");
    dispose();
  });

  test("delivers a hex colour frame to the callback", () => {
    const seen: (string | null)[] = [];
    const dispose = openLocalColorWatch((c) => seen.push(c));
    latest().open();
    latest().message(JSON.stringify({ color: "#e58c4d" }));
    expect(seen).toEqual(["#e58c4d"]);
    dispose();
  });

  test("delivers a null colour frame (library colour cleared)", () => {
    const seen: (string | null)[] = [];
    const dispose = openLocalColorWatch((c) => seen.push(c));
    latest().message(JSON.stringify({ color: null }));
    expect(seen).toEqual([null]);
    dispose();
  });

  test("drops a malformed frame without throwing or calling back", () => {
    const seen: (string | null)[] = [];
    const dispose = openLocalColorWatch((c) => seen.push(c));
    expect(() => latest().message("not json")).not.toThrow();
    expect(seen).toEqual([]);
    dispose();
  });

  test("the disposer closes the socket and defuses the reconnect", () => {
    const dispose = openLocalColorWatch(() => {});
    const ws = latest();
    dispose();
    expect(ws.readyState).toBe(3); // CLOSED
    // Handlers defused before close → a queued onclose can't schedule a reconnect.
    expect(ws.onclose).toBeNull();
  });
});
