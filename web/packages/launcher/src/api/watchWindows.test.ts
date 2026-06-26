// The live window-feed watch is the launcher's resync channel: it must reconnect
// when the socket drops so a row can't strand on a stale starting/connecting
// status (the dangling spinner). A fake WebSocket + fake timers drive the
// reconnect-with-backoff path; the reconnected socket's on-connect snapshot is
// the consolidation step that re-syncs the world.

import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { liveApi } from "./library";

class FakeWS {
  static instances: FakeWS[] = [];
  onmessage: ((ev: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  closedByCaller = false;
  constructor(public url: string) {
    FakeWS.instances.push(this);
  }
  close(): void {
    this.closedByCaller = true;
    this.onclose?.();
  }
}

beforeEach(() => {
  FakeWS.instances = [];
  vi.useFakeTimers();
  vi.stubGlobal("WebSocket", FakeWS as unknown as typeof WebSocket);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

function frame(ws: FakeWS, windows: unknown[]): void {
  ws.onmessage?.({ data: JSON.stringify({ windows }) });
}

describe("liveApi.watchWindows reconnect", () => {
  it("reconnects after the socket closes and resyncs from the new snapshot", () => {
    const seen: number[] = [];
    const unsub = liveApi.watchWindows((s) => seen.push(s.windows.length));
    expect(FakeWS.instances.length).toBe(1);

    // First connection delivers a snapshot.
    frame(FakeWS.instances[0]!, []);
    expect(seen).toEqual([0]);

    // Socket drops: a reconnect is SCHEDULED (not immediate), then fires.
    FakeWS.instances[0]!.onclose?.();
    expect(FakeWS.instances.length).toBe(1);
    vi.advanceTimersByTime(500);
    expect(FakeWS.instances.length).toBe(2);

    // The reconnect's on-connect snapshot re-syncs the world.
    frame(FakeWS.instances[1]!, [{}, {}]);
    expect(seen.at(-1)).toBe(2);

    unsub();
  });

  it("backs off exponentially across repeated failures, capped", () => {
    const unsub = liveApi.watchWindows(() => {});
    // 1st drop -> 500ms
    FakeWS.instances[0]!.onclose?.();
    vi.advanceTimersByTime(499);
    expect(FakeWS.instances.length).toBe(1);
    vi.advanceTimersByTime(1);
    expect(FakeWS.instances.length).toBe(2);
    // 2nd drop (no healthy frame) -> 1000ms
    FakeWS.instances[1]!.onclose?.();
    vi.advanceTimersByTime(999);
    expect(FakeWS.instances.length).toBe(2);
    vi.advanceTimersByTime(1);
    expect(FakeWS.instances.length).toBe(3);
    unsub();
  });

  it("resets the backoff after a healthy frame", () => {
    const unsub = liveApi.watchWindows(() => {});
    FakeWS.instances[0]!.onclose?.();
    vi.advanceTimersByTime(500);
    expect(FakeWS.instances.length).toBe(2);
    // A frame proves the link healthy: the next drop reconnects at 500 again.
    frame(FakeWS.instances[1]!, []);
    FakeWS.instances[1]!.onclose?.();
    vi.advanceTimersByTime(500);
    expect(FakeWS.instances.length).toBe(3);
    unsub();
  });

  it("stops reconnecting once unsubscribed", () => {
    const unsub = liveApi.watchWindows(() => {});
    unsub(); // sets stopped + closes; onclose must NOT schedule
    vi.advanceTimersByTime(60000);
    expect(FakeWS.instances.length).toBe(1);
  });
});
