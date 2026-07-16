// @vitest-environment jsdom
//
// The PTY socket's liveness kit: the app-level heartbeat (client
// {"type":"ping"} -> server {"type":"pong"}, the watcher vocabulary), the
// read-deadline that force-closes a half-open zombie, and the capped-backoff
// redial through the existing session/since/generation reattach. The kit
// shares the watcher's constants from transport.ts (source-pinned below so
// the two cannot drift); the live 300s gateway-cut proof rides the host
// smoke + the gateway rig.

import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import TerminalTab from "./TerminalTab.svelte";
import terminalSource from "./TerminalTab.svelte?raw";
import docSyncSource from "../state/docSync.svelte.ts?raw";
import sceneSyncSource from "../state/sceneSync.svelte.ts?raw";
import {
  WS_PING_MS,
  WS_READ_DEADLINE_MS,
  WS_RECONNECT_BACKOFF_MIN_MS,
  WS_RECONNECT_BACKOFF_MAX_MS,
} from "../api/transport";
import type { TerminalTab as TerminalTabState } from "../state/tabs.svelte";

const mounted: Array<Record<string, any>> = [];
const sockets: TestWebSocket[] = [];

class TestResizeObserver {
  observe() {}
  disconnect() {}
}

class TestWebSocket {
  static OPEN = 1;

  readyState = 0;
  binaryType = "blob";
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: unknown }) => void | Promise<void>) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  sent: string[] = [];

  constructor(readonly url: string) {
    sockets.push(this);
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = 3;
    this.onclose?.();
  }

  open() {
    this.readyState = TestWebSocket.OPEN;
    this.onopen?.();
  }

  // A dial that fails before ever opening (connection refused).
  failDial() {
    this.readyState = 3;
    this.onclose?.();
  }

  pings(): number {
    return this.sent.filter((s) => s === JSON.stringify({ type: "ping" })).length;
  }
}

// Lines the component writes INTO the terminal (term.writeln): the surface
// the version-skew guard must keep ping-error spam out of.
const writtenLines = vi.hoisted(() => [] as string[]);

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 80;
    rows = 24;
    options: Record<string, unknown> = {};

    loadAddon() {}
    open() {}
    attachCustomKeyEventHandler() {}
    onData() {}
    onResize() {}
    write() {}
    writeln(line: string) {
      writtenLines.push(line);
    }
    resize(cols: number, rows: number) {
      this.cols = cols;
      this.rows = rows;
    }
    focus() {}
    dispose() {}
  },
}));

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
  },
}));

vi.mock("@xterm/addon-search", () => ({
  SearchAddon: class {
    findNext() {}
    findPrevious() {}
  },
}));

vi.mock("@xterm/addon-serialize", () => ({
  SerializeAddon: class {
    serialize() {
      return "";
    }
  },
}));

vi.mock("@xterm/addon-web-links", () => ({
  WebLinksAddon: class {},
}));

globalThis.ResizeObserver = TestResizeObserver as any;
globalThis.WebSocket = TestWebSocket as any;
globalThis.requestAnimationFrame = ((cb: FrameRequestCallback) => {
  cb(0);
  return 0;
}) as any;
HTMLCanvasElement.prototype.getContext = (() => ({})) as any;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  sockets.splice(0);
  writtenLines.splice(0);
  document.body.innerHTML = "";
  vi.useRealTimers();
});

function terminalTab(partial: Partial<TerminalTabState> = {}): TerminalTabState {
  return {
    kind: "terminal",
    id: "term-hb-1",
    title: "Terminal",
    createdAt: 1,
    broadcastEnabled: false,
    broadcastTargetIds: [],
    ...partial,
  };
}

async function renderTerminal(tab: TerminalTabState) {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(TerminalTab, {
    target,
    props: { tab, paneId: "pane-1", active: true, focused: false },
  });
  mounted.push(component);
  await tick();
  await tick();
  return target;
}

function lastSocket(): TestWebSocket {
  const socket = sockets.at(-1);
  if (!socket) throw new Error("expected terminal websocket");
  return socket;
}

async function attach(socket: TestWebSocket, id = "sess-1"): Promise<void> {
  socket.open();
  await socket.onmessage?.({
    data: JSON.stringify({
      type: "session",
      id,
      seq: 0,
      generation: 1,
      missed_bytes: 0,
      bytes_since_focus: 0,
    }),
  });
}

async function pong(socket: TestWebSocket): Promise<void> {
  await socket.onmessage?.({ data: JSON.stringify({ type: "pong" }) });
}

describe("terminal heartbeat", () => {
  test("pings every WS_PING_MS while the socket is open", async () => {
    await renderTerminal(terminalTab());
    const socket = lastSocket();
    await attach(socket);

    expect(socket.pings()).toBe(0);
    await vi.advanceTimersByTimeAsync(WS_PING_MS);
    expect(socket.pings()).toBe(1);
    await pong(socket);
    await vi.advanceTimersByTimeAsync(WS_PING_MS);
    expect(socket.pings()).toBe(2);
    await pong(socket);
    // Frames kept arriving, so the read-deadline never tripped: one socket.
    expect(sockets).toHaveLength(1);
    expect(socket.readyState).toBe(TestWebSocket.OPEN);
  });

  test("a silent socket trips the read-deadline and redials the SAME session", async () => {
    const tab = terminalTab();
    await renderTerminal(tab);
    const socket = lastSocket();
    await attach(socket, "sess-keep");

    // No inbound frames at all: pings go out unanswered and the deadline
    // force-closes the zombie.
    await vi.advanceTimersByTimeAsync(WS_READ_DEADLINE_MS);
    expect(socket.readyState).toBe(3);
    expect(sockets).toHaveLength(1);

    // The redial fires after the first backoff step and reattaches by id.
    await vi.advanceTimersByTimeAsync(WS_RECONNECT_BACKOFF_MIN_MS);
    expect(sockets).toHaveLength(2);
    expect(lastSocket().url).toContain("session=sess-keep");
    expect(tab.terminalSessionId).toBe("sess-keep");

    // A successful reattach resumes the heartbeat on the new socket.
    await attach(lastSocket(), "sess-keep");
    await vi.advanceTimersByTimeAsync(WS_PING_MS);
    expect(lastSocket().pings()).toBe(1);
  });

  test("exactly one redial is in flight after a deadline trip", async () => {
    await renderTerminal(terminalTab());
    await attach(lastSocket());

    await vi.advanceTimersByTimeAsync(WS_READ_DEADLINE_MS);
    // Far past every timer: the one scheduled redial fires; the un-opened
    // dial arms no heartbeat, so nothing else ever closes or re-dials.
    await vi.advanceTimersByTimeAsync(60_000);
    expect(sockets).toHaveLength(2);
  });

  test("redial backoff doubles per failure and caps at the max", async () => {
    await renderTerminal(terminalTab());
    await attach(lastSocket());

    // Trip the deadline, then fail every dial the moment it is scheduled in.
    await vi.advanceTimersByTimeAsync(WS_READ_DEADLINE_MS);
    // Delays consumed: 500 (the trip's redial), then doubling per failed dial.
    const delays = [500, 1000, 2000, 4000, 8000, 8000, 8000];
    for (const delay of delays) {
      const count = sockets.length;
      await vi.advanceTimersByTimeAsync(delay - 1);
      expect(sockets.length).toBe(count);
      await vi.advanceTimersByTimeAsync(1);
      expect(sockets.length).toBe(count + 1);
      lastSocket().failDial();
    }
  });

  test("the session id survives failed redials until the attach budget spends", async () => {
    const tab = terminalTab({ terminalSessionId: "sess-durable" });
    await renderTerminal(tab);

    // The mount dial + 3 more failures = 4 consecutive attach failures:
    // the resumable session must survive every one of them.
    lastSocket().failDial();
    for (let i = 0; i < 3; i++) {
      await vi.advanceTimersByTimeAsync(WS_RECONNECT_BACKOFF_MAX_MS);
      lastSocket().failDial();
    }
    expect(tab.terminalSessionId).toBe("sess-durable");

    // The 5th consecutive failure spends the budget: the id drops so the
    // NEXT dial starts a fresh session instead of redialing a dead one.
    await vi.advanceTimersByTimeAsync(WS_RECONNECT_BACKOFF_MAX_MS);
    lastSocket().failDial();
    expect(tab.terminalSessionId).toBeUndefined();
  });

  test("an old server's unknown-variant ping error is liveness, not terminal spam", async () => {
    await renderTerminal(terminalTab());
    const socket = lastSocket();
    await attach(socket);

    await socket.onmessage?.({
      data: JSON.stringify({
        type: "error",
        message: "invalid terminal frame: unknown variant `ping`, expected one of `input`",
      }),
    });
    expect(writtenLines.some((l) => l.includes("invalid terminal frame"))).toBe(false);

    // A real error still writes into the terminal.
    await socket.onmessage?.({
      data: JSON.stringify({ type: "error", message: "pty write failed" }),
    });
    expect(writtenLines.some((l) => l.includes("terminal error: pty write failed"))).toBe(true);
  });
});

describe("heartbeat source pins", () => {
  test("the kit rides the shared transport constants, no duplicated literals", () => {
    expect(terminalSource).toContain(
      'pingTimer = setInterval(() => send({ type: "ping" }), WS_PING_MS);',
    );
    expect(terminalSource).toMatch(/deadlineTimer = setTimeout\([\s\S]{0,200}WS_READ_DEADLINE_MS\)/);
    expect(terminalSource).toContain(
      "reconnectBackoffMs = Math.min(reconnectBackoffMs * 2, WS_RECONNECT_BACKOFF_MAX_MS);",
    );
    expect(terminalSource).toContain("let reconnectBackoffMs = WS_RECONNECT_BACKOFF_MIN_MS;");
    // The single-dial guard: an explicit connect supersedes a scheduled
    // redial before the socket teardown.
    expect(terminalSource).toMatch(
      /async function connect\(\): Promise<void> \{\n    if \(!term\) return;\n    \/\/ Single-dial guard/,
    );
    expect(terminalSource).toContain("cancelReconnect();");
  });

  test("doc-sync and scene-sync backoffs ride the same shared constants", () => {
    for (const source of [docSyncSource, sceneSyncSource]) {
      expect(source).toContain("private backoffMs = WS_RECONNECT_BACKOFF_MIN_MS;");
      expect(source).toContain(
        "this.backoffMs = Math.min(this.backoffMs * 2, WS_RECONNECT_BACKOFF_MAX_MS);",
      );
      // No local literals left to drift.
      expect(source).not.toMatch(/RECONNECT_BASE_MS|RECONNECT_MAX_MS/);
    }
  });
});
