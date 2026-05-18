// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import {
  __testSetBootstrapHydrated,
  __testApplyOverlaysFromHash,
  graphOverlay,
  graphReloadSignal,
  onWatchEvent,
  openFsGraphForDirectory,
  openFsGraphForFile,
  persistStateToHash,
  scheduleSessionSave,
  scopeFsGraphFromHere,
  settingsOverlay,
} from "./store.svelte";
import { layout, type LeafNode, type TerminalTab } from "./tabs.svelte";

function setTerminalLayout(tab: Partial<TerminalTab> = {}): void {
  const terminal: TerminalTab = {
    kind: "terminal",
    id: "term-1",
    title: "Terminal",
    createdAt: 1,
    broadcastEnabled: false,
    broadcastTargetIds: [],
    ...tab,
  };
  const pane: LeafNode = {
    kind: "leaf",
    id: "pane-test",
    tabs: [terminal],
    activeTabId: terminal.id,
  };
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
}

afterEach(() => {
  __testSetBootstrapHydrated(true);
  const pane: LeafNode = {
    kind: "leaf",
    id: "pane-reset",
    tabs: [],
    activeTabId: null,
  };
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  graphOverlay.open = false;
  graphOverlay.mode = "semantic";
  graphOverlay.scopeId = "drive";
  graphOverlay.depth = 1;
  graphOverlay.filters.link = true;
  graphOverlay.filters.tag = true;
  graphOverlay.filters.mention = true;
  graphOverlay.filters.language = true;
  graphOverlay.filters.img = true;
  graphOverlay.filters.folder = true;
  graphOverlay.inspectorOpen = false;
  graphOverlay.pendingSelectId = null;
  settingsOverlay.open = false;
  graphReloadSignal.nonce = 0;
  window.history.replaceState(null, "", "/");
});

describe("session persistence bootstrap guard", () => {
  test("does not save a tsid-less layout while bootstrap hydration is pending", async () => {
    vi.useFakeTimers();
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(null, {
        status: 204,
      }),
    );
    setTerminalLayout({ terminalSessionId: undefined, lastSeq: undefined });

    __testSetBootstrapHydrated(false);
    scheduleSessionSave();
    await vi.runAllTimersAsync();
    expect(fetchSpy).not.toHaveBeenCalled();

    activeTerminal().terminalSessionId = "term_after_hydrate";
    activeTerminal().lastSeq = 12;
    __testSetBootstrapHydrated(true);
    scheduleSessionSave();
    await vi.runAllTimersAsync();

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    const [, init] = fetchSpy.mock.calls[0]!;
    expect(String(init?.body)).toContain("term_after_hydrate");

    fetchSpy.mockRestore();
    vi.useRealTimers();
  });
});

function activeTerminal(): TerminalTab {
  const node = layout.nodes[layout.activePaneId];
  if (!node || node.kind !== "leaf") throw new Error("expected active leaf");
  const tab = node.tabs[0];
  if (!tab || tab.kind !== "terminal") throw new Error("expected terminal tab");
  return tab;
}

describe("graph watcher reload signal", () => {
  test("increments only while the graph overlay is open", async () => {
    vi.useFakeTimers();
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = input instanceof Request ? input.url : String(input);
      const body = url.includes("/api/graph")
        ? { nodes: [], edges: [] }
        : url.includes("/api/drive")
          ? { name: "test", root: "/tmp/test", preferences: {} }
          : [];
      return new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });

    graphOverlay.open = false;
    onWatchEvent({ kind: "modified", event: { path: "notes/a.md" } });
    await Promise.resolve();
    expect(graphReloadSignal.nonce).toBe(0);

    graphOverlay.open = true;
    onWatchEvent({ kind: "modified", event: { path: "notes/a.md" } });
    await Promise.resolve();
    expect(graphReloadSignal.nonce).toBe(1);

    vi.clearAllTimers();
    fetchSpy.mockRestore();
    vi.useRealTimers();
  });
});

describe("graph overlay hash persistence", () => {
  test("filesystem graph mode is encoded only when needed", () => {
    window.history.replaceState(null, "", "/");
    graphOverlay.open = true;
    graphOverlay.mode = "filesystem";
    graphOverlay.scopeId = "dir:src";
    graphOverlay.depth = 1;

    persistStateToHash();

    expect(decodeURIComponent(window.location.hash)).toBe("#graph=dir:src|1||0|fs");
  });

  test("filesystem graph mode restores from the optional hash token", () => {
    window.history.replaceState(null, "", "/#graph=file:src/app.ts|2||0|fs");
    graphOverlay.mode = "semantic";
    graphOverlay.scopeId = "drive";
    graphOverlay.depth = 1;

    __testApplyOverlaysFromHash();

    expect(graphOverlay.open).toBe(true);
    expect(graphOverlay.mode).toBe("filesystem");
    expect(graphOverlay.scopeId).toBe("file:src/app.ts");
    expect(graphOverlay.depth).toBe(2);
    expect(graphOverlay.inspectorOpen).toBe(false);
  });

  test("legacy graph hashes default back to semantic mode", () => {
    window.history.replaceState(null, "", "/#graph=file:README.md|3");
    graphOverlay.mode = "filesystem";

    __testApplyOverlaysFromHash();

    expect(graphOverlay.mode).toBe("semantic");
    expect(graphOverlay.scopeId).toBe("file:README.md");
    expect(graphOverlay.depth).toBe(3);
  });

  test("persistence strips unknown legacy hash keys and keeps live keys", () => {
    const removedOverlayKey = "assist" + "ant";
    window.history.replaceState(
      null,
      "",
      `/#${removedOverlayKey}=open&scopes=2&settings=1`,
    );
    settingsOverlay.open = true;

    persistStateToHash();

    expect(window.location.hash).toBe("#settings=1");
  });
});

describe("filesystem graph entrypoints", () => {
  test("file browser graph entrypoints open drive-scope fs graph with a preselection", () => {
    openFsGraphForFile("notes/a.md");

    expect(graphOverlay.open).toBe(true);
    expect(graphOverlay.mode).toBe("filesystem");
    expect(graphOverlay.scopeId).toBe("drive");
    expect(graphOverlay.pendingSelectId).toBe("notes/a.md");

    openFsGraphForDirectory("notes");

    expect(graphOverlay.open).toBe(true);
    expect(graphOverlay.mode).toBe("filesystem");
    expect(graphOverlay.scopeId).toBe("drive");
    expect(graphOverlay.pendingSelectId).toBe("notes");
  });

  test("filesystem graph scope action pivots to files and directories", () => {
    scopeFsGraphFromHere("notes", true);

    expect(graphOverlay.open).toBe(true);
    expect(graphOverlay.mode).toBe("filesystem");
    expect(graphOverlay.scopeId).toBe("dir:notes");
    expect(graphOverlay.depth).toBe(1);
    expect(graphOverlay.pendingSelectId).toBe("notes");

    scopeFsGraphFromHere("notes/a.md", false);

    expect(graphOverlay.mode).toBe("filesystem");
    expect(graphOverlay.scopeId).toBe("file:notes/a.md");
    expect(graphOverlay.depth).toBe(1);
    expect(graphOverlay.pendingSelectId).toBe("notes/a.md");
  });
});
