// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import {
  __testApplyTreeExpandedReloadSnapshot,
  __testSetBootstrapHydrated,
  __testApplyOverlaysFromHash,
  browserSelection,
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
  expandAllFoldersForInstance,
  collapseAllFoldersForInstance,
  isFullyExpandedForInstance,
  revealAndSelect,
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
  settingsOverlay.open = false;
  graphReloadSignal.nonce = 0;
  browserSelection.path = null;
  browserSelection.showWorkspace = false;
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
  test("increments only while a graph tab is open", async () => {
    vi.useFakeTimers();
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = input instanceof Request ? input.url : String(input);
      const body = url.includes("/api/graph")
        ? { nodes: [], edges: [] }
        : url.includes("/api/workspace")
          ? { name: "test", root: "/tmp/test", preferences: {} }
          : [];
      return new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });

    // No graph tab open: the watcher signal stays put. (After the
    // scope-concept wipe the reload gate is `hasGraphTab()`, not a
    // graphOverlay flag.)
    const empty: LeafNode = { kind: "leaf", id: "p", tabs: [], activeTabId: null };
    layout.rootId = "p";
    layout.activePaneId = "p";
    layout.nodes = { p: empty };
    onWatchEvent({ kind: "modified", event: { path: "notes/a.md" } });
    await Promise.resolve();
    expect(graphReloadSignal.nonce).toBe(0);

    // Open a graph tab: now watcher events bump the reload signal.
    openFsGraphForDirectory("");
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

describe("legacy overlay hash retirement (W5)", () => {
  test("legacy graph= / files= bookmarks are ignored on restore", () => {
    // The scope-concept wipe retired the per-overlay hash for the graph +
    // browser surfaces; those are first-class tabs restored via the layout
    // `s` key now. Old `graph=` / `files=` bookmarks must degrade
    // gracefully - never reopen the (now-dead) overlays, never crash.
    window.history.replaceState(
      null,
      "",
      "/#graph=file:README.md|3|||fs&files=1:notes.md",
    );
    // Must not throw and must not open any graph/browser surface: the
    // legacy keys are no longer in HASH_KEYS, so they're ignored (and
    // the overlay state they used to workspace is gone entirely).
    expect(() => __testApplyOverlaysFromHash()).not.toThrow();
    expect(
      activePane().tabs.some((t) => t.kind === "graph" || t.kind === "browser"),
    ).toBe(false);
  });

  test("persistence strips retired + unknown legacy hash keys, keeps live keys", () => {
    // `assistant` (phase-5) + `scopes` are unknown; `graph` + `files` are
    // now retired (W5). All fall out of HASH_KEYS, so dropUnknownHashKeys
    // strips them while the live `settings` key survives.
    const removedOverlayKey = "assist" + "ant";
    window.history.replaceState(
      null,
      "",
      `/#${removedOverlayKey}=open&scopes=2&graph=workspace&files=1:notes.md&settings=1`,
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
    expect(graph.mode).toBe("filesystem");
    // File trigger scopes to the parent directory, with the
    // originating file auto-selected so its inspector pops.
    expect(graph.scopeId).toBe("dir:notes");
    expect(graph.pendingSelectId).toBe("notes/a.md");

    openFsGraphForDirectory("notes");

    graph = activeGraphTab();
    expect(graph.mode).toBe("filesystem");
    // Directory trigger scopes to that subtree directly.
    expect(graph.scopeId).toBe("dir:notes");
    expect(graph.pendingSelectId).toBe("notes");
  });

  test("file at workspace root falls back to workspace scope; workspace root directory likewise", () => {
    openFsGraphForFile("README.md");

    let graph = activeGraphTab();
    expect(graph.mode).toBe("filesystem");
    // No parent directory above a root-level file: workspace scope is
    // the meaningful neighbourhood.
    expect(graph.scopeId).toBe("workspace");
    expect(graph.pendingSelectId).toBe("README.md");

    openFsGraphForDirectory("");

    graph = activeGraphTab();
    expect(graph.mode).toBe("filesystem");
    expect(graph.scopeId).toBe("workspace");
  });

  test("filesystem graph scope action pivots to files and directories", () => {
    scopeFsGraphFromHere("notes", true);

    let graph = activeGraphTab();
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

describe("external-change banner (lane-c addendum-2 item 1)", () => {
  function placeFileTab(path: string, content: string): FileTab {
    const tab: FileTab = {
      kind: "file",
      fileKind: "document",
      id: `file-${path}`,
      path,
      content,
      saved: content,
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
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-ext",
      tabs: [tab],
      activeTabId: tab.id,
    };
    layout.rootId = pane.id;
    layout.activePaneId = pane.id;
    layout.nodes = { [pane.id]: pane };
    // Return the LIVE (proxied) tab from the layout $state, not the raw
    // literal - flagExternalChange mutates through the proxy.
    return (layout.nodes[pane.id] as LeafNode).tabs[0] as FileTab;
  }

  test("a watch event flags the open tab but never reloads or clears its content", () => {
    // Regression: the watcher used to silently reload a clean buffer,
    // which replaced the doc and snapped the caret to line 1, col 1 while
    // @@Alex was typing. The watch path now only raises the dismissable
    // banner (externalChange) and leaves the buffer untouched.
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = input instanceof Request ? input.url : String(input);
      const body = url.includes("/api/workspace")
        ? { name: "test", root: "/tmp/test", preferences: {} }
        : [];
      return new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    const tab = placeFileTab("notes/a.md", "hello, still typing");

    onWatchEvent({ kind: "modified", event: { path: "notes/a.md" } });

    // flagExternalChange runs synchronously in the watch loop.
    expect(tab.externalChange).toBe(true);
    // The buffer is untouched: no silent reload, the caret stays put.
    expect(tab.content).toBe("hello, still typing");

    fetchSpy.mockRestore();
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

  test("empty pane falls back to workspace root", () => {
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

  test("root-level file -> workspace root + file", () => {
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

  test("terminal without cwd -> workspace root", () => {
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

  test("browser with no selection -> workspace root", () => {
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

  test("graph workspace scope -> workspace root", () => {
    placeTabs([
      {
        kind: "graph",
        id: "g-1",
        title: "Graph",
        mode: "semantic",
        scopeId: "workspace",
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

  test("graph tag: scope -> workspace root (no useful path anchor)", () => {
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

// Phase-12 Slice E: FileTree renders + toggles off the per-instance map, so
// the expand-all / collapse-all / full-expansion helpers now target one
// instance. A dock side and a tab (two instances) must not toggle each other;
// a programmatic reveal, by contrast, fans out to every live surface.
describe("per-instance expansion helpers (Slice E)", () => {
  function seedTree(): void {
    const dirs = ["docs", "docs/api", "notes"];
    tree.entries = dirs.map(
      (path): TreeEntry => ({
        path,
        is_dir: true,
        size: 0,
        mtime: null,
      }),
    );
  }

  test("expand-all / collapse-all target only the named instance", () => {
    seedTree();
    const dock = ensureFbTreeInstance("fb-dock-left");
    const tab = ensureFbTreeInstance("fb-tab-1");

    expandAllFoldersForInstance("fb-dock-left");
    expect(dock.expanded).toEqual({
      "": true,
      docs: true,
      "docs/api": true,
      notes: true,
    });
    // The sibling instance is untouched: independence is the fix.
    expect(tab.expanded).toEqual({ "": true });
    expect(isFullyExpandedForInstance("fb-dock-left")).toBe(true);
    expect(isFullyExpandedForInstance("fb-tab-1")).toBe(false);

    collapseAllFoldersForInstance("fb-dock-left");
    expect(dock.expanded).toEqual({ "": true });
    expect(isFullyExpandedForInstance("fb-dock-left")).toBe(false);
  });

  test("isFullyExpandedForInstance is false for an unregistered instance", () => {
    seedTree();
    expect(isFullyExpandedForInstance("fb-overlay")).toBe(false);
  });

  test("revealAndSelect fans ancestor expansion across all live instances", () => {
    const dock = ensureFbTreeInstance("fb-dock-left");
    const tab = ensureFbTreeInstance("fb-tab-1");

    revealAndSelect("docs/api/spec.md");

    // Both surfaces reveal the new entry's ancestor chain (not the file),
    // matching the pre-migration "all surfaces share a reveal" behavior.
    expect(dock.expanded).toEqual({ "": true, docs: true, "docs/api": true });
    expect(tab.expanded).toEqual({ "": true, docs: true, "docs/api": true });
    expect(browserSelection.path).toBe("docs/api/spec.md");
  });
});
