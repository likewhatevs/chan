// @vitest-environment jsdom

// Dedicated launcher-theme watch. Pins that it opens a WebSocket to
// /api/library/local-theme/watch, delivers `{ theme }` frames (dark / light /
// null) to the callback, drops malformed frames, and the disposer closes the
// socket + defuses the reconnect. A structural twin of localColorWatch.test.ts.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { openLocalThemeWatch } from "./transport";

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

describe("openLocalThemeWatch", () => {
  test("opens a WS to the local-theme watch path", () => {
    const dispose = openLocalThemeWatch(() => {});
    expect(latest().url).toContain("/api/library/local-theme/watch");
    dispose();
  });

  test("delivers a theme frame to the callback", () => {
    const seen: (string | null)[] = [];
    const dispose = openLocalThemeWatch((t) => seen.push(t));
    latest().open();
    latest().message(JSON.stringify({ theme: "light" }));
    expect(seen).toEqual(["light"]);
    dispose();
  });

  test("delivers a null theme frame (follow OS)", () => {
    const seen: (string | null)[] = [];
    const dispose = openLocalThemeWatch((t) => seen.push(t));
    latest().message(JSON.stringify({ theme: null }));
    expect(seen).toEqual([null]);
    dispose();
  });

  test("drops a malformed frame without throwing or calling back", () => {
    const seen: (string | null)[] = [];
    const dispose = openLocalThemeWatch((t) => seen.push(t));
    expect(() => latest().message("not json")).not.toThrow();
    expect(seen).toEqual([]);
    dispose();
  });

  test("the disposer closes the socket and defuses the reconnect", () => {
    const dispose = openLocalThemeWatch(() => {});
    const ws = latest();
    dispose();
    expect(ws.readyState).toBe(3); // CLOSED
    expect(ws.onclose).toBeNull();
  });
});
