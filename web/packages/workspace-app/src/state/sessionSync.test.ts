// @vitest-environment jsdom

// Live co-view session sync: the `session_changed` /ws frame filter and
// the refetch -> reconcile -> echo-suppression pipeline in store.svelte.
// The wire contract (frame shape, nonce echo, omitted `client`, deleted
// variant) is pinned server-side in chan-server's sessions tests; these
// tests pin the receiver's behavior against that contract.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

vi.mock("./caretIndex");

import { api, clientNonce, sessionWindowId } from "../api/client";
import {
  __testResetSessionDiscarded,
  __testSetBootstrapHydrated,
  discardWindowSessionLocal,
  onWatchEvent,
  scheduleSessionSave,
} from "./store.svelte";
import { layout, type FileTab, type LeafNode } from "./tabs.svelte";

function fileTab(partial: Partial<FileTab> = {}): FileTab {
  return {
    kind: "file",
    fileKind: "document",
    id: "file-1",
    path: "notes/a.md",
    content: "saved",
    saved: "saved",
    savedMtime: 1,
    mode: "wysiwyg",
    loading: false,
    error: null,
    fileMissing: null,
    inspectorOpen: false,
    outlineOpen: false,
    repoRoot: null,
    readMode: false,
    fsWritable: true,
    styleToolbarOpen: false,
    syntaxHighlight: true,
    highlightTrailingWhitespace: false,
    codeBlocksCollapsed: false,
    ...partial,
  };
}

function resetLayout(): LeafNode {
  const tab = fileTab();
  const pane: LeafNode = {
    kind: "leaf",
    id: "pane-sync",
    tabs: [tab],
    activeTabId: tab.id,
  };
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  layout.focusColor = "blue";
  return layout.nodes[pane.id] as LeafNode;
}

/// A remote payload congruent with `resetLayout()`'s tree, carrying one
/// observable in-place change (focus color green).
function remotePayload(): unknown {
  return { layout: { k: "l", t: [{ p: "notes/a.md", m: "wysiwyg" }], wc: "g" } };
}

function fireFrame(frame: Record<string, unknown>): void {
  onWatchEvent({ kind: "session_changed", ...frame });
}

beforeEach(() => {
  vi.useFakeTimers();
  resetLayout();
  // Normalize module state: clear any pending save timer + the dedupe
  // snapshot, then re-arm saves.
  discardWindowSessionLocal();
  __testResetSessionDiscarded();
  __testSetBootstrapHydrated(true);
});

afterEach(async () => {
  // Drain armed sync/save timers while the api mocks are still in place,
  // so no callback leaks into the next test's clock.
  await vi.runOnlyPendingTimersAsync();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

describe("session_changed frame filter", () => {
  test("a foreign-nonce frame for this window refetches and reconciles", async () => {
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());

    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    expect(getSession).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(250);

    expect(getSession).toHaveBeenCalledTimes(1);
    expect(layout.focusColor).toBe("green");
  });

  test("a frame without a client nonce is treated as foreign", async () => {
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());

    fireFrame({ w: sessionWindowId() });
    await vi.advanceTimersByTimeAsync(250);

    expect(getSession).toHaveBeenCalledTimes(1);
  });

  test("drops own-nonce echoes", async () => {
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());

    fireFrame({ w: sessionWindowId(), client: clientNonce() });
    await vi.advanceTimersByTimeAsync(1000);

    expect(getSession).not.toHaveBeenCalled();
  });

  test("drops frames for another window", async () => {
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());

    fireFrame({ w: "someone-elses-window", client: "peer-nonce" });
    await vi.advanceTimersByTimeAsync(1000);

    expect(getSession).not.toHaveBeenCalled();
  });

  test("drops deleted frames (a peer's blob delete never tears down live tabs)", async () => {
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());

    fireFrame({ w: sessionWindowId(), client: "peer-nonce", deleted: true });
    await vi.advanceTimersByTimeAsync(1000);

    expect(getSession).not.toHaveBeenCalled();
  });

  test("drops frames before bootstrap hydration", async () => {
    __testSetBootstrapHydrated(false);
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());

    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    await vi.advanceTimersByTimeAsync(1000);

    expect(getSession).not.toHaveBeenCalled();
    __testSetBootstrapHydrated(true);
  });

  test("a frame burst coalesces into one refetch", async () => {
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());

    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    await vi.advanceTimersByTimeAsync(1000);

    expect(getSession).toHaveBeenCalledTimes(1);
  });
});

describe("session sync apply pipeline", () => {
  test("a clean apply pre-seeds the save dedupe so the reactive echo save no-ops", async () => {
    vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());
    const putSession = vi.spyOn(api, "putSession").mockResolvedValue(undefined);

    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    await vi.advanceTimersByTimeAsync(250);
    expect(layout.focusColor).toBe("green");

    // The layout mutation retriggers App.svelte's save walk; simulate it.
    scheduleSessionSave();
    await vi.advanceTimersByTimeAsync(750);

    expect(putSession).not.toHaveBeenCalled();
  });

  test("a diverged apply leaves the snapshot unseeded so the local save pushes back", async () => {
    // Remote tree has an extra tab: structurally diverged from the live
    // single-tab pane, so reconcile refuses.
    vi.spyOn(api, "getSession").mockResolvedValue({
      layout: {
        k: "l",
        t: [{ p: "notes/a.md" }, { p: "notes/other.md" }],
        wc: "g",
      },
    });
    const putSession = vi.spyOn(api, "putSession").mockResolvedValue(undefined);
    vi.spyOn(console, "warn").mockImplementation(() => {});

    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    await vi.advanceTimersByTimeAsync(250);
    expect(layout.focusColor).toBe("blue");

    scheduleSessionSave();
    await vi.advanceTimersByTimeAsync(750);

    expect(putSession).toHaveBeenCalledTimes(1);
  });

  test("an inbound frame racing a pending local save flushes the save and refetches after", async () => {
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(remotePayload());
    const putSession = vi.spyOn(api, "putSession").mockResolvedValue(undefined);

    // Local edit sits in the 750ms debounce...
    scheduleSessionSave();
    // ...when a peer's write lands.
    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    await vi.advanceTimersByTimeAsync(250);

    // The pending save was flushed instead of applying the remote blob.
    expect(putSession).toHaveBeenCalledTimes(1);
    expect(getSession).not.toHaveBeenCalled();
    expect(layout.focusColor).toBe("blue");

    // The re-armed refetch then observes post-flush state and applies a
    // genuinely newer peer write.
    await vi.advanceTimersByTimeAsync(250);
    expect(getSession).toHaveBeenCalledTimes(1);
    expect(layout.focusColor).toBe("green");
  });

  test("an echo of the blob this window already carries applies nothing", async () => {
    const putSession = vi.spyOn(api, "putSession").mockResolvedValue(undefined);

    // Save once so lastSessionSnapshot holds our serialization.
    scheduleSessionSave();
    await vi.advanceTimersByTimeAsync(750);
    expect(putSession).toHaveBeenCalledTimes(1);
    const saved = JSON.parse(JSON.stringify(putSession.mock.calls[0]![0]));

    // The server hands back exactly that blob.
    const getSession = vi.spyOn(api, "getSession").mockResolvedValue(saved);
    fireFrame({ w: sessionWindowId(), client: "peer-nonce" });
    await vi.advanceTimersByTimeAsync(250);

    expect(getSession).toHaveBeenCalledTimes(1);
    // No state change and no re-save from the no-op apply.
    scheduleSessionSave();
    await vi.advanceTimersByTimeAsync(750);
    expect(putSession).toHaveBeenCalledTimes(1);
  });
});
