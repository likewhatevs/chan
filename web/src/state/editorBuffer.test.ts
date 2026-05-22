import { afterEach, beforeAll, beforeEach, describe, expect, test, vi } from "vitest";

// `fullstack-a-72`: vitest's jsdom env doesn't ship localStorage
// in this setup (vitest 4 + jsdom 29 quirk; window exists but
// window.localStorage is undefined). Inline a minimal in-memory
// Storage polyfill so the tests can exercise the real buffer
// module — same shape the browser provides + handles the Storage
// quota by throwing on a 5MB cap (matches typical browsers + the
// module's prune-then-retry logic).
beforeAll(() => {
  if (typeof globalThis.localStorage !== "undefined") return;
  let map = new Map<string, string>();
  const storage: Storage = {
    get length() { return map.size; },
    clear() { map = new Map(); },
    getItem(key: string) { return map.get(key) ?? null; },
    key(i: number) { return Array.from(map.keys())[i] ?? null; },
    removeItem(key: string) { map.delete(key); },
    setItem(key: string, value: string) { map.set(key, value); },
  };
  Object.defineProperty(globalThis, "localStorage", { value: storage, configurable: true });
});
import {
  bufferKey,
  clearEditorBuffer,
  divergentBufferOrNull,
  pruneEditorBuffers,
  readEditorBuffer,
  writeEditorBuffer,
} from "./editorBuffer";

// `fullstack-a-72` hang-recovery via localStorage. These tests
// exercise the buffer module in isolation (no Svelte component
// involvement): write/read/clear roundtrips, TTL eviction,
// size-cap eviction, malformed-entry recovery, and the
// divergent-buffer helper used by FileEditorTab.

const MS_PER_DAY = 24 * 60 * 60 * 1000;

beforeEach(() => {
  // Fresh localStorage per test so prior buffer entries don't
  // bleed into eviction counts.
  if (typeof localStorage !== "undefined") {
    localStorage.clear();
  }
});

afterEach(() => {
  vi.useRealTimers();
});

describe("fullstack-a-72: editorBuffer roundtrip", () => {
  test("write + read returns the buffer", () => {
    writeEditorBuffer("tab-1", "hello", "notes/a.md");
    const buf = readEditorBuffer("tab-1");
    expect(buf).not.toBeNull();
    expect(buf!.content).toBe("hello");
    expect(buf!.path).toBe("notes/a.md");
    expect(typeof buf!.updatedAt).toBe("number");
  });

  test("read returns null when no entry", () => {
    expect(readEditorBuffer("tab-missing")).toBeNull();
  });

  test("clear removes the entry", () => {
    writeEditorBuffer("tab-1", "hello", "notes/a.md");
    clearEditorBuffer("tab-1");
    expect(readEditorBuffer("tab-1")).toBeNull();
  });

  test("bufferKey uses the expected namespace prefix", () => {
    expect(bufferKey("tab-1")).toBe("chan:editor-buffer:tab-1");
  });
});

describe("fullstack-a-72: malformed-entry recovery", () => {
  test("read returns null + clears entry when JSON is malformed", () => {
    localStorage.setItem(bufferKey("tab-bad"), "not-json");
    expect(readEditorBuffer("tab-bad")).toBeNull();
    expect(localStorage.getItem(bufferKey("tab-bad"))).toBeNull();
  });

  test("read returns null + clears entry when shape is wrong", () => {
    localStorage.setItem(
      bufferKey("tab-bad"),
      JSON.stringify({ content: "ok", updatedAt: "not-a-number" }),
    );
    expect(readEditorBuffer("tab-bad")).toBeNull();
    expect(localStorage.getItem(bufferKey("tab-bad"))).toBeNull();
  });
});

describe("fullstack-a-72: TTL eviction in pruneEditorBuffers", () => {
  test("entries older than 7 days are evicted", () => {
    // Hand-craft a stale entry (8 days old).
    const eightDaysAgo = Date.now() - 8 * MS_PER_DAY;
    localStorage.setItem(
      bufferKey("stale"),
      JSON.stringify({ content: "old", updatedAt: eightDaysAgo, path: "x.md" }),
    );
    // Fresh entry within TTL.
    writeEditorBuffer("fresh", "new", "y.md");

    const evicted = pruneEditorBuffers();
    expect(evicted).toBe(1);
    expect(readEditorBuffer("stale")).toBeNull();
    expect(readEditorBuffer("fresh")).not.toBeNull();
  });

  test("entries within TTL are kept", () => {
    writeEditorBuffer("recent", "fresh", "x.md");
    const evicted = pruneEditorBuffers();
    expect(evicted).toBe(0);
    expect(readEditorBuffer("recent")).not.toBeNull();
  });

  test("non-buffer localStorage entries are not touched", () => {
    localStorage.setItem("unrelated-key", "value");
    writeEditorBuffer("recent", "fresh", "x.md");
    pruneEditorBuffers();
    expect(localStorage.getItem("unrelated-key")).toBe("value");
  });
});

describe("fullstack-a-72: divergentBufferOrNull helper", () => {
  test("returns null when no buffer exists", () => {
    expect(divergentBufferOrNull("tab-1", "notes/a.md", "disk")).toBeNull();
  });

  test("returns null when buffer content matches disk content (clean state)", () => {
    writeEditorBuffer("tab-1", "same", "notes/a.md");
    expect(divergentBufferOrNull("tab-1", "notes/a.md", "same")).toBeNull();
  });

  test("returns buffer when content diverges from disk", () => {
    writeEditorBuffer("tab-1", "unsaved", "notes/a.md");
    const buf = divergentBufferOrNull("tab-1", "notes/a.md", "disk");
    expect(buf).not.toBeNull();
    expect(buf!.content).toBe("unsaved");
  });

  test("clears + returns null when buffer's path doesn't match the tab's current path", () => {
    // Defensive: a tab-id collision with a different path
    // shouldn't restore stale wrong-file content.
    writeEditorBuffer("tab-1", "from-other-file", "notes/old.md");
    expect(divergentBufferOrNull("tab-1", "notes/new.md", "disk")).toBeNull();
    // The mismatched-path entry should have been cleared so the
    // next read also returns null.
    expect(readEditorBuffer("tab-1")).toBeNull();
  });
});
