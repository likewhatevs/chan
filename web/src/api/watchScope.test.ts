// @vitest-environment jsdom

// Phase-11 Slice A: the watcher socket gained a client -> server scope
// subscription path (`sub` / `unsub` frames) on top of the existing
// server -> client stream. These tests pin the exact wire shape the server's
// `ScopeRegistry` expects and the open/reconnect re-subscribe contract, with
// a controllable fake WebSocket (jsdom has no WebSocket).

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { openWatchSocket } from "./client";
import type { WsClientFrame } from "./types";

class FakeWebSocket {
  static OPEN = 1;
  static instances: FakeWebSocket[] = [];

  readyState = 0; // CONNECTING
  sent: string[] = [];
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

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = 3; // CLOSED
  }
}

beforeEach(() => {
  FakeWebSocket.instances = [];
  vi.stubGlobal("WebSocket", FakeWebSocket as unknown as typeof WebSocket);
  window.history.replaceState(null, "", "/");
});

afterEach(() => {
  vi.unstubAllGlobals();
});

function latest(): FakeWebSocket {
  const ws = FakeWebSocket.instances.at(-1);
  if (!ws) throw new Error("no socket opened");
  return ws;
}

function parseSent(ws: FakeWebSocket): WsClientFrame[] {
  return ws.sent.map((s) => JSON.parse(s) as WsClientFrame);
}

describe("watcher socket scope subscription", () => {
  test("subscribeDir / unsubscribeDir serialize the exact wire frames", () => {
    const sub = openWatchSocket(() => {});
    const ws = latest();
    ws.open();

    sub.subscribeDir("notes/recipes");
    sub.unsubscribeDir("notes/recipes");
    sub.subscribeDir(""); // workspace root scope

    expect(parseSent(ws)).toEqual([
      { type: "sub", dir: "notes/recipes" },
      { type: "unsub", dir: "notes/recipes" },
      { type: "sub", dir: "" },
    ]);

    sub.close();
  });

  test("frames queued before open are dropped (no buffering)", () => {
    const sub = openWatchSocket(() => {});
    const ws = latest();
    // Socket still CONNECTING; the owner re-subscribes from onReady, so a
    // pre-open frame must not be buffered or replayed.
    sub.subscribeDir("notes");
    expect(ws.sent).toEqual([]);

    ws.open();
    sub.subscribeDir("notes");
    expect(parseSent(ws)).toEqual([{ type: "sub", dir: "notes" }]);

    sub.close();
  });

  test("onReady fires on connect so the owner can re-establish scopes", () => {
    const ready = vi.fn();
    const sub = openWatchSocket(() => {}, undefined, ready);
    expect(ready).not.toHaveBeenCalled();

    latest().open();
    expect(ready).toHaveBeenCalledTimes(1);

    sub.close();
  });

  test("send after close is a no-op (socket no longer OPEN)", () => {
    const sub = openWatchSocket(() => {});
    const ws = latest();
    ws.open();
    sub.close();

    sub.subscribeDir("notes");
    expect(ws.sent).toEqual([]);
  });

  test("the handle is callable as the disposer (back-compat)", () => {
    const sub = openWatchSocket(() => {});
    const ws = latest();
    ws.open();
    sub(); // dispose via call
    expect(ws.readyState).toBe(3); // CLOSED
  });
});
