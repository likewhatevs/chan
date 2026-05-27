// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import {
  __testApplyTreeExpandedReloadSnapshot,
  __testSetBootstrapHydrated,
  __testApplyOverlaysFromHash,
  browserOverlay,
  browserSelection,
  graphOverlay,
  graphReloadSignal,
  onWatchEvent,
  openFsGraphForDirectory,
  openFsGraphForFile,
  persistStateToHash,
  persistTreeExpanded,
  resolveSpawnContext,
  revealPathInBrowser,
  scheduleSessionSave,
  scopeFsGraphFromHere,
  settingsOverlay,
  tree,
  treeExpanded,
  fbTreeInstances,
  ensureFbTreeInstance,
  fbTreeInstance,
  disposeFbTreeInstance,
  fbDirSubscriberCount,
} from "./store.svelte";
import {
  activePane,
  layout,
  paneMode,
  type BrowserTab,
  type FileTab,
  type GraphTab,
  type LeafNode,
  type TerminalTab,
} from "./tabs.svelte";
import type { TreeEntry } from "../api/types";

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
  browserOverlay.open = false;
  browserSelection.path = null;
  browserSelection.showDrive = false;
  tree.loadedDirs = {};
  tree.loadingDirs = {};
  tree.dirErrors = {};
  treeExpanded.map = { "": true };
  fbTreeInstances.byId = {};
  window.sessionStorage.clear();
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

describe("file browser expansion reload persistence", () => {
  test("mirrors expanded directories into sessionStorage for same-screen reload", () => {
    vi.useFakeTimers();
    window.history.replaceState(null, "", "/");
    treeExpanded.map = {
      "": true,
      docs: true,
      "docs/api": true,
      collapsed: false,
    };

    persistTreeExpanded();
    treeExpanded.map = { "": true };

    expect(__testApplyTreeExpandedReloadSnapshot()).toBe(true);
    expect(treeExpanded.map).toEqual({
      "": true,
      docs: true,
      "docs/api": true,
    });

    vi.clearAllTimers();
    vi.useRealTimers();
  });

  test("reload restore mutates the existing expansion map in place", () => {
    window.history.replaceState(null, "", "/");
    treeExpanded.map = { "": true, docs: true };
    persistTreeExpanded();

    const captured = treeExpanded.map;
    delete captured.docs;
    captured[""] = true;

    expect(__testApplyTreeExpandedReloadSnapshot()).toBe(true);
    expect(treeExpanded.map).toBe(captured);
    expect(captured[""]).toBe(true);
    expect(captured.docs).toBe(true);
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

describe("window commands", () => {
  test("open_browser enter expands the target directory", () => {
    window.history.replaceState(null, "", "/?w=window-a");
    tree.loadedDirs = { "notes/sub": true };

    onWatchEvent({
      type: "window_command",
      window_id: "window-a",
      command: "open_browser",
      path: "notes/sub",
      enter: true,
    });

    expect(browserOverlay.open).toBe(false);
    expect(browserSelection.path).toBe("notes/sub");
    expect(activePane().tabs.some((tab) => tab.kind === "browser")).toBe(true);
    expect(treeExpanded.map[""]).toBe(true);
    expect(treeExpanded.map["notes"]).toBe(true);
    expect(treeExpanded.map["notes/sub"]).toBe(true);
  });

  test("open_browser select keeps file selection behavior", () => {
    window.history.replaceState(null, "", "/?w=window-a");

    onWatchEvent({
      type: "window_command",
      window_id: "window-a",
      command: "open_browser",
      path: "notes",
      select: "notes/photo.png",
    });

    expect(browserOverlay.open).toBe(false);
    expect(browserSelection.path).toBe("notes/photo.png");
    expect(activePane().tabs.some((tab) => tab.kind === "browser")).toBe(true);
    expect(treeExpanded.map[""]).toBe(true);
    expect(treeExpanded.map["notes"]).toBe(true);
  });

  test("revealPathInBrowser always OPENS a File Browser tab (never focuses the dock / an existing tab)", () => {
    // @@Alex: with a docked File Browser, reveal-in-browser must never
    // focus the dock (or silently reuse another pane's browser tab) - it
    // always OPENS a File Browser tab in the active pane. (Was: "focuses
    // an existing browser tab instead of duplicating it"; that reuse is
    // the behavior being overridden - it was the GI-8 root cause where a
    // reveal from a graph tab produced no visible tab.)
    const leftBrowser: BrowserTab = {
      kind: "browser",
      id: "browser-left",
      title: "Files",
      inspectorOpen: false,
    };
    const left: LeafNode = {
      kind: "leaf",
      id: "pane-left",
      tabs: [leftBrowser],
      activeTabId: leftBrowser.id,
    };
    const rightTerminal: TerminalTab = {
      kind: "terminal",
      id: "term-right",
      title: "Terminal",
      createdAt: 1,
      broadcastEnabled: false,
      broadcastTargetIds: [],
    };
    const right: LeafNode = {
      kind: "leaf",
      id: "pane-right",
      tabs: [rightTerminal],
      activeTabId: rightTerminal.id,
    };
    layout.rootId = "root";
    layout.activePaneId = right.id;
    layout.nodes = {
      root: { kind: "split", id: "root", direction: "row", a: left.id, b: right.id, ratio: 0.5 },
      [left.id]: left,
      [right.id]: right,
    };
    // openBrowserInActivePane resolves the active pane via activeLayout();
    // ensure pane-mode isn't active so it reads this layout, not a draft.
    paneMode.active = false;

    const tab = revealPathInBrowser("notes/today.md", { inspectorOpen: true });

    // A NEW browser tab opens in the ACTIVE pane (right); the existing
    // leftBrowser is left untouched (not reused/focused). Read panes back
    // from `layout.nodes` (the $state proxy) - assigning `layout.nodes`
    // deep-proxies the panes, so the push lands on the proxy, not the raw
    // local consts.
    const leftAfter = layout.nodes[left.id] as LeafNode;
    const rightAfter = layout.nodes[right.id] as LeafNode;
    expect(tab.kind).toBe("browser");
    expect(tab.id).not.toBe(leftBrowser.id);
    expect(layout.activePaneId).toBe(right.id);
    expect(leftAfter.tabs).toHaveLength(1);
    expect(rightAfter.tabs).toHaveLength(2);
    expect(rightAfter.activeTabId).toBe(tab.id);
    expect(tab.selected).toBe("notes/today.md");
    expect(tab.expanded).toEqual(["notes"]);
    expect(tab.inspectorOpen).toBe(true);
    expect(browserSelection.path).toBe("notes/today.md");
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
  function activeGraphTab(): GraphTab {
    const tab = activePane().tabs.find((candidate) => candidate.id === activePane().activeTabId);
    expect(tab?.kind).toBe("graph");
    return tab as GraphTab;
  }

  test("file browser graph entrypoints scope to the file's parent or the directory itself", () => {
    openFsGraphForFile("notes/a.md");

    let graph = activeGraphTab();
    expect(graphOverlay.open).toBe(false);
    expect(graph.mode).toBe("filesystem");
    // File trigger scopes to the parent directory, with the
    // originating file auto-selected so its inspector pops.
    expect(graph.scopeId).toBe("dir:notes");
    expect(graph.pendingSelectId).toBe("notes/a.md");

    openFsGraphForDirectory("notes");

    graph = activeGraphTab();
    expect(graphOverlay.open).toBe(false);
    expect(graph.mode).toBe("filesystem");
    // Directory trigger scopes to that subtree directly.
    expect(graph.scopeId).toBe("dir:notes");
    expect(graph.pendingSelectId).toBe("notes");
  });

  test("file at drive root falls back to drive scope; drive root directory likewise", () => {
    openFsGraphForFile("README.md");

    let graph = activeGraphTab();
    expect(graph.mode).toBe("filesystem");
    // No parent directory above a root-level file: drive scope is
    // the meaningful neighbourhood.
    expect(graph.scopeId).toBe("drive");
    expect(graph.pendingSelectId).toBe("README.md");

    openFsGraphForDirectory("");

    graph = activeGraphTab();
    expect(graph.mode).toBe("filesystem");
    expect(graph.scopeId).toBe("drive");
  });

  test("filesystem graph scope action pivots to files and directories", () => {
    scopeFsGraphFromHere("notes", true);

    let graph = activeGraphTab();
    expect(graphOverlay.open).toBe(false);
    expect(graph.mode).toBe("filesystem");
    expect(graph.scopeId).toBe("dir:notes");
    expect(graph.depth).toBe(1);
    expect(graph.pendingSelectId).toBe("notes");

    scopeFsGraphFromHere("notes/a.md", false);

    graph = activeGraphTab();
    expect(graph.mode).toBe("filesystem");
    expect(graph.scopeId).toBe("file:notes/a.md");
    expect(graph.depth).toBe(1);
    expect(graph.pendingSelectId).toBe("notes/a.md");
  });
});

describe("resolveSpawnContext (fullstack-43)", () => {
  function placeTabs(tabs: Array<FileTab | TerminalTab | GraphTab | BrowserTab>): void {
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-spawn",
      tabs,
      activeTabId: tabs[0]?.id ?? null,
    };
    layout.rootId = pane.id;
    layout.activePaneId = pane.id;
    layout.nodes = { [pane.id]: pane };
  }

  function makeFileTab(path: string): FileTab {
    return {
      kind: "file",
      fileKind: "document",
      id: `file-${path}`,
      path,
      content: "",
      saved: "",
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
    };
  }

  test("empty pane falls back to drive root", () => {
    placeTabs([]);
    expect(resolveSpawnContext()).toEqual({ dir: "" });
  });

  test("file editor source -> parent dir + file", () => {
    placeTabs([makeFileTab("notes/sub/a.md")]);
    expect(resolveSpawnContext()).toEqual({
      dir: "notes/sub",
      file: "notes/sub/a.md",
    });
  });

  test("root-level file -> drive root + file", () => {
    placeTabs([makeFileTab("README.md")]);
    expect(resolveSpawnContext()).toEqual({ dir: "", file: "README.md" });
  });

  test("terminal with cwd -> cwd as dir", () => {
    const terminal: TerminalTab = {
      kind: "terminal",
      id: "term-1",
      title: "Terminal",
      createdAt: 1,
      broadcastEnabled: false,
      broadcastTargetIds: [],
      cwd: "notes/sub",
    };
    placeTabs([terminal]);
    expect(resolveSpawnContext()).toEqual({ dir: "notes/sub" });
  });

  test("terminal without cwd -> drive root", () => {
    const terminal: TerminalTab = {
      kind: "terminal",
      id: "term-1",
      title: "Terminal",
      createdAt: 1,
      broadcastEnabled: false,
      broadcastTargetIds: [],
    };
    placeTabs([terminal]);
    expect(resolveSpawnContext()).toEqual({ dir: "" });
  });

  test("browser selection of a file -> parent + file", () => {
    placeTabs([
      { kind: "browser", id: "br-1", title: "Files", inspectorOpen: false },
    ]);
    tree.entries = [
      { path: "notes/a.md", is_dir: false, mtime: 1, size: 0 } as TreeEntry,
    ];
    browserSelection.path = "notes/a.md";
    expect(resolveSpawnContext()).toEqual({
      dir: "notes",
      file: "notes/a.md",
    });
  });

  test("browser selection of a directory -> dir only", () => {
    placeTabs([
      { kind: "browser", id: "br-1", title: "Files", inspectorOpen: false },
    ]);
    tree.entries = [
      { path: "notes", is_dir: true, mtime: null, size: 0 } as TreeEntry,
    ];
    browserSelection.path = "notes";
    expect(resolveSpawnContext()).toEqual({ dir: "notes" });
  });

  test("browser with no selection -> drive root", () => {
    placeTabs([
      { kind: "browser", id: "br-1", title: "Files", inspectorOpen: false },
    ]);
    browserSelection.path = null;
    expect(resolveSpawnContext()).toEqual({ dir: "" });
  });

  test("graph file: scope -> parent + file", () => {
    placeTabs([
      {
        kind: "graph",
        id: "g-1",
        title: "File Graph",
        mode: "semantic",
        scopeId: "file:notes/a.md",
        depth: 1,
        filters: {
          link: true,
          tag: true,
          mention: true,
          language: true,
          img: true,
          folder: true,
          markdown: true,
          source: true,
        },
        inspectorOpen: false,
        pendingSelectId: null,
      },
    ]);
    expect(resolveSpawnContext()).toEqual({
      dir: "notes",
      file: "notes/a.md",
    });
  });

  test("graph dir: scope -> dir only", () => {
    placeTabs([
      {
        kind: "graph",
        id: "g-1",
        title: "Dir Graph",
        mode: "semantic",
        scopeId: "dir:notes/sub",
        depth: 1,
        filters: {
          link: true,
          tag: true,
          mention: true,
          language: true,
          img: true,
          folder: true,
          markdown: true,
          source: true,
        },
        inspectorOpen: false,
        pendingSelectId: null,
      },
    ]);
    expect(resolveSpawnContext()).toEqual({ dir: "notes/sub" });
  });

  test("graph drive scope -> drive root", () => {
    placeTabs([
      {
        kind: "graph",
        id: "g-1",
        title: "Graph",
        mode: "semantic",
        scopeId: "drive",
        depth: 1,
        filters: {
          link: true,
          tag: true,
          mention: true,
          language: true,
          img: true,
          folder: true,
          markdown: true,
          source: true,
        },
        inspectorOpen: false,
        pendingSelectId: null,
      },
    ]);
    expect(resolveSpawnContext()).toEqual({ dir: "" });
  });

  test("graph tag: scope -> drive root (no useful path anchor)", () => {
    placeTabs([
      {
        kind: "graph",
        id: "g-1",
        title: "Tag Graph",
        mode: "semantic",
        scopeId: "tag:foo",
        depth: 1,
        filters: {
          link: true,
          tag: true,
          mention: true,
          language: true,
          img: true,
          folder: true,
          markdown: true,
          source: true,
        },
        inspectorOpen: false,
        pendingSelectId: null,
      },
    ]);
    expect(resolveSpawnContext()).toEqual({ dir: "" });
  });
});

// Phase-11 Slice A: per-File-Browser-instance tree metadata. The registry is
// the keyed structure that lets two simultaneously-visible instances keep
// independent expand/collapse state (round-1 ask). These tests pin the
// create/read/dispose contract and the cross-instance subscription refcount.
describe("per-instance file browser tree registry", () => {
  test("ensureFbTreeInstance creates a root-expanded record, idempotent", () => {
    const a = ensureFbTreeInstance("pane-1");
    expect(a.expanded).toEqual({ "": true });
    expect(a.subscribedDirs).toEqual({ "": true });
    expect(a.selected).toBeNull();

    a.expanded["notes"] = true;
    // Re-ensure returns the SAME record (no clobber on remount).
    const again = ensureFbTreeInstance("pane-1");
    expect(again).toBe(a);
    expect(again.expanded["notes"]).toBe(true);
  });

  test("instances are independent: expanding one does not touch another", () => {
    const a = ensureFbTreeInstance("pane-1");
    const b = ensureFbTreeInstance("pane-2");
    a.expanded["docs"] = true;
    expect(b.expanded["docs"]).toBeUndefined();
    expect(fbTreeInstance("pane-2")?.expanded).toEqual({ "": true });
  });

  test("fbTreeInstance returns null for an unknown id", () => {
    expect(fbTreeInstance("nope")).toBeNull();
  });

  test("disposeFbTreeInstance forgets the record", () => {
    ensureFbTreeInstance("pane-1");
    expect(fbTreeInstance("pane-1")).not.toBeNull();
    disposeFbTreeInstance("pane-1");
    expect(fbTreeInstance("pane-1")).toBeNull();
    // Disposing an unknown id is a safe no-op.
    expect(() => disposeFbTreeInstance("pane-1")).not.toThrow();
  });

  test("fbDirSubscriberCount is the cross-instance refcount", () => {
    const a = ensureFbTreeInstance("pane-1");
    const b = ensureFbTreeInstance("pane-2");
    // Both implicitly subscribe to the root scope.
    expect(fbDirSubscriberCount("")).toBe(2);
    expect(fbDirSubscriberCount("notes")).toBe(0);

    a.subscribedDirs["notes"] = true;
    expect(fbDirSubscriberCount("notes")).toBe(1);
    b.subscribedDirs["notes"] = true;
    expect(fbDirSubscriberCount("notes")).toBe(2);

    // Last instance to drop the dir takes the count back to zero (the
    // transition Slice E maps to an `unsub` frame).
    delete a.subscribedDirs["notes"];
    delete b.subscribedDirs["notes"];
    expect(fbDirSubscriberCount("notes")).toBe(0);

    disposeFbTreeInstance("pane-2");
    expect(fbDirSubscriberCount("")).toBe(1);
  });
});
