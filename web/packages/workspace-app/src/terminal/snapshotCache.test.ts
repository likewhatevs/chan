import { beforeAll, beforeEach, describe, expect, test } from "vitest";

// vitest's jsdom env here doesn't ship localStorage; inline a minimal in-memory
// Storage polyfill so the tests exercise the real module (mirrors
// editorBuffer.test.ts).
beforeAll(() => {
  if (typeof globalThis.localStorage !== "undefined") return;
  let map = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return map.size;
    },
    clear() {
      map = new Map();
    },
    getItem(key: string) {
      return map.get(key) ?? null;
    },
    key(i: number) {
      return Array.from(map.keys())[i] ?? null;
    },
    removeItem(key: string) {
      map.delete(key);
    },
    setItem(key: string, value: string) {
      map.set(key, value);
    },
  };
  Object.defineProperty(globalThis, "localStorage", {
    value: storage,
    configurable: true,
  });
});

import {
  clearTerminalSnapshot,
  MAX_ONE_SNAPSHOT_BYTES,
  pruneTerminalSnapshots,
  readTerminalSnapshot,
  type TerminalSnapshot,
  writeTerminalSnapshot,
} from "./snapshotCache";

const MS_PER_DAY = 24 * 60 * 60 * 1000;

function snap(over: Partial<TerminalSnapshot> = {}): TerminalSnapshot {
  return {
    ansi: "\x1b[2J\x1b[Hhello",
    generation: 1,
    lastSeq: 42,
    cols: 80,
    rows: 24,
    updatedAt: Date.now(),
    ...over,
  };
}

beforeEach(() => {
  if (typeof localStorage !== "undefined") localStorage.clear();
});

describe("snapshotCache", () => {
  test("round-trips a snapshot keyed by session id", () => {
    writeTerminalSnapshot("term_a", snap({ lastSeq: 99, generation: 3 }));
    const got = readTerminalSnapshot("term_a");
    expect(got).not.toBeNull();
    expect(got?.lastSeq).toBe(99);
    expect(got?.generation).toBe(3);
    expect(got?.cols).toBe(80);
    expect(readTerminalSnapshot("term_missing")).toBeNull();
  });

  test("drops a capture over the per-snapshot byte budget", () => {
    const huge = "x".repeat(MAX_ONE_SNAPSHOT_BYTES + 1);
    writeTerminalSnapshot("term_big", snap({ ansi: huge }));
    expect(readTerminalSnapshot("term_big")).toBeNull();
  });

  test("malformed entries read as null and are cleared", () => {
    localStorage.setItem("chan:term-snapshot:term_bad", "not-json");
    expect(readTerminalSnapshot("term_bad")).toBeNull();
    expect(localStorage.getItem("chan:term-snapshot:term_bad")).toBeNull();
  });

  test("clear removes the entry", () => {
    writeTerminalSnapshot("term_c", snap());
    clearTerminalSnapshot("term_c");
    expect(readTerminalSnapshot("term_c")).toBeNull();
  });

  test("prune evicts entries past the TTL", () => {
    writeTerminalSnapshot("term_old", snap({ updatedAt: Date.now() - 4 * MS_PER_DAY }));
    writeTerminalSnapshot("term_new", snap({ updatedAt: Date.now() }));
    const evicted = pruneTerminalSnapshots();
    expect(evicted).toBeGreaterThanOrEqual(1);
    expect(readTerminalSnapshot("term_old")).toBeNull();
    expect(readTerminalSnapshot("term_new")).not.toBeNull();
  });

  test("prune enforces the total cap oldest-first and spares foreign prefixes", () => {
    // Five ~120KB snapshots = ~600KB > the 512KB total cap; the oldest evicts.
    const big = "y".repeat(120 * 1024);
    // Recent, strictly-increasing timestamps (term_0 oldest) so the TTL pass
    // keeps all five and only the size-cap pass evicts the oldest.
    for (let i = 0; i < 5; i++) {
      writeTerminalSnapshot(
        `term_${i}`,
        snap({ ansi: big, updatedAt: Date.now() - (5 - i) * 1000 }),
      );
    }
    // Other features' stores must NEVER be touched by our prune.
    localStorage.setItem("chan:editor-buffer:keepme", "editor");
    localStorage.setItem("chan:caret-index:keepme", "caret");
    pruneTerminalSnapshots();
    expect(readTerminalSnapshot("term_0")).toBeNull(); // oldest gone
    expect(readTerminalSnapshot("term_4")).not.toBeNull(); // newest kept
    expect(localStorage.getItem("chan:editor-buffer:keepme")).toBe("editor");
    expect(localStorage.getItem("chan:caret-index:keepme")).toBe("caret");
  });
});

// Pre-guard builds snapshotted the control terminal's scrollback, which always
// contains the CHAN_DEVSERVER_TOKEN= marker: a devserver credential at rest.
// The load-time sweep drops any such entry unconditionally. Red mutation: make
// the credential branch of pruneTerminalSnapshots a no-op.
describe("credential sweep", () => {
  test("the load-time sweep drops a snapshot carrying the token marker", () => {
    writeTerminalSnapshot(
      "ctl_session",
      snap({ ansi: "connect...\r\nCHAN_DEVSERVER_TOKEN=tok_leaked\r\n$ " }),
    );
    writeTerminalSnapshot("plain_session", snap());
    expect(readTerminalSnapshot("ctl_session")).not.toBeNull();
    expect(pruneTerminalSnapshots()).toBeGreaterThanOrEqual(1);
    expect(readTerminalSnapshot("ctl_session")).toBeNull();
    expect(readTerminalSnapshot("plain_session")).not.toBeNull();
  });

  test("a fresh, in-TTL entry carrying the marker is dropped too (age is no defense)", () => {
    writeTerminalSnapshot(
      "ctl_fresh",
      snap({
        ansi: "CHAN_DEVSERVER_TOKEN=tok_now",
        updatedAt: Date.now(),
      }),
    );
    pruneTerminalSnapshots();
    expect(readTerminalSnapshot("ctl_fresh")).toBeNull();
  });

  test("after the sweep, no snapshot on disk contains the token marker", () => {
    // The at-rest security property itself, not a proxy for it.
    writeTerminalSnapshot(
      "ctl_a",
      snap({ ansi: "x CHAN_DEVSERVER_TOKEN=tok_a" }),
    );
    writeTerminalSnapshot(
      "ctl_b",
      snap({ ansi: "CHAN_DEVSERVER_TOKEN=tok_b trailing" }),
    );
    writeTerminalSnapshot("plain", snap());
    pruneTerminalSnapshots();
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (!key || !key.startsWith("chan:term-snapshot:")) continue;
      expect(localStorage.getItem(key) ?? "").not.toContain(
        "CHAN_DEVSERVER_TOKEN=",
      );
    }
  });
});
