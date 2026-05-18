// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import { api } from "../api/client";
import { confirmState, resolveConfirm } from "./confirm.svelte";
import { editorToolsPrefs } from "./editorTools.svelte";
import {
  activePane,
  broadcastTerminalInput,
  canReopenClosedTab,
  clearRecentlyClosedTabsForTest,
  closeTab,
  dismissTerminalEnvNamePrompt,
  focusColorForPane,
  hydrateTerminalSessionsFromLayout,
  layout,
  openInPane,
  openFind,
  openTerminalInPane,
  removeTerminalFromBroadcastGroup,
  registerTerminalInputSink,
  markLocalTabDrop,
  renameTerminalTab,
  reopenClosedTab,
  reorderTab,
  restoreLayout,
  saveTab,
  scheduleAutosave,
  serializeLayout,
  setTerminalBroadcastMuted,
  setTerminalBroadcastTarget,
  setPaneFocusColor,
  setTerminalSession,
  shouldCloseTabAfterDragEnd,
  splitPane,
  tabLabelInPane,
  terminalBroadcastMemberIds,
  terminalEnvTabNameStale,
  toggleAllTerminalBroadcastMuted,
  type FileTab,
  type LeafNode,
  type TerminalTab,
} from "./tabs.svelte";

function resetLayout(tabs: Array<FileTab | TerminalTab>): LeafNode {
  const pane: LeafNode = {
    kind: "leaf",
    id: "pane-test",
    tabs,
    activeTabId: tabs[0]?.id ?? null,
    focusColor: "blue",
  };
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  return pane;
}

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

function terminalTab(partial: Partial<TerminalTab> = {}): TerminalTab {
  return {
    kind: "terminal",
    id: "term-1",
    title: "Terminal",
    createdAt: 1,
    broadcastEnabled: false,
    broadcastTargetIds: [],
    ...partial,
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
  resolveConfirm(false);
  resetLayout([]);
  clearRecentlyClosedTabsForTest();
  editorToolsPrefs.stripTrailingWhitespaceOnSave = false;
});

describe("tab close confirmation", () => {
  test("keeps a dirty file tab open when confirmation is cancelled", async () => {
    const tab = fileTab({ content: "unsaved" });
    const pane = resetLayout([tab]);

    const close = closeTab(pane.id, tab.id);
    expect(confirmState.open).toBe(true);
    expect(confirmState.message).toContain("unsaved changes");
    resolveConfirm(false);
    await close;

    expect(activePane().tabs).toHaveLength(1);
  });

  test("closes a dirty file tab after confirmation", async () => {
    const tab = fileTab({ content: "unsaved" });
    const pane = resetLayout([tab]);

    const close = closeTab(pane.id, tab.id);
    expect(confirmState.open).toBe(true);
    resolveConfirm(true);
    await close;

    expect(activePane().tabs).toHaveLength(0);
  });

  test("prompts for live terminal tabs", async () => {
    const tab = terminalTab();
    const pane = resetLayout([tab]);
    const unregister = registerTerminalInputSink(tab.id, () => {});

    const close = closeTab(pane.id, tab.id);
    expect(confirmState.open).toBe(true);
    expect(confirmState.message).toContain("still running");
    resolveConfirm(false);
    await close;

    unregister();
    expect(activePane().tabs).toHaveLength(1);
  });

  test("reopens a closed dirty file tab with its in-memory buffer", async () => {
    const tab = fileTab({ content: "unsaved", saved: "saved", caret: { from: 3, to: 3 } });
    const pane = resetLayout([tab]);

    await closeTab(pane.id, tab.id, { force: true });
    expect(activePane().tabs).toHaveLength(0);
    expect(canReopenClosedTab()).toBe(true);

    expect(reopenClosedTab()).toBe(true);
    expect(activePane().tabs).toHaveLength(1);
    const reopened = activePane().tabs[0];
    expect(reopened?.kind).toBe("file");
    if (reopened?.kind !== "file") return;
    expect(reopened.content).toBe("unsaved");
    expect(reopened.saved).toBe("saved");
    expect(reopened.caret).toEqual({ from: 3, to: 3 });
    expect(activePane().activeTabId).toBe(reopened.id);
  });
});

describe("tab drag and drop", () => {
  test("same-pane drag onto adjacent inactive tab reorders without closing source", () => {
    const active = fileTab({ id: "file-active", path: "notes/active.md" });
    const inactive = fileTab({ id: "file-inactive", path: "notes/inactive.md" });
    const pane = resetLayout([active, inactive]);
    pane.activeTabId = active.id;

    markLocalTabDrop(pane.id, active.id);
    reorderTab(pane.id, active.id, 1);

    expect(activePane().tabs.map((tab) => tab.id)).toEqual([inactive.id, active.id]);
    expect(activePane().activeTabId).toBe(active.id);
    expect(shouldCloseTabAfterDragEnd(pane.id, active.id, "move")).toBe(false);
    expect(activePane().tabs.map((tab) => tab.id)).toEqual([inactive.id, active.id]);
  });
});

describe("pane state", () => {
  test("serializes per-pane focus color with layout state", async () => {
    const pane = resetLayout([terminalTab()]);
    setPaneFocusColor(pane.id, "pink");

    const snapshot = serializeLayout();
    expect(JSON.stringify(snapshot)).toContain("\"pc\":\"p\"");

    await restoreLayout(snapshot!);

    expect(focusColorForPane(activePane().id)).toBe("pink");
  });

  test("can split before the active pane for left/up menu actions", () => {
    const pane = resetLayout([fileTab()]);

    splitPane(pane.id, "row", "before");

    const root = layout.nodes[layout.rootId];
    expect(root?.kind).toBe("split");
    if (root?.kind !== "split") return;
    expect(root.direction).toBe("row");
    expect(layout.nodes[root.a]?.kind).toBe("leaf");
    expect(root.b).toBe(pane.id);
    expect(layout.activePaneId).toBe(root.a);
  });
});

describe("find state", () => {
  test("reopening an already open find bar bumps the focus nonce", () => {
    const tab = fileTab();
    resetLayout([tab]);

    openFind(tab.id);
    const opened = activePane().tabs[0] as FileTab;
    expect(opened.find?.open).toBe(true);
    expect(opened.find?.focusNonce).toBe(1);

    openFind(tab.id);
    expect(opened.find?.open).toBe(true);
    expect(opened.find?.focusNonce).toBe(2);
  });
});

describe("terminal session serialization", () => {
  test("keeps terminal session ids out of shareable layout hashes", () => {
    resetLayout([
      terminalTab({
        terminalSessionId: "term_123",
        lastSeq: 99,
      }),
    ]);

    const layoutSnapshot = serializeLayout();

    expect(JSON.stringify(layoutSnapshot)).not.toContain("term_123");
    expect(JSON.stringify(layoutSnapshot)).not.toContain("99");
  });

  test("round-trips terminal session ids through session layout payloads", async () => {
    resetLayout([
      terminalTab({
        title: "build",
        terminalSessionId: "term_123",
        lastSeq: 99,
      }),
    ]);
    const layoutSnapshot = serializeLayout({ terminalSessions: true });

    await restoreLayout(layoutSnapshot!);

    const [tab] = activePane().tabs;
    expect(tab?.kind).toBe("terminal");
    if (tab?.kind !== "terminal") return;
    expect(tab.title).toBe("build");
    expect(tab.mcpEnv).toBe(true);
    expect(tab.sessionMcpEnv).toBe(true);
    expect(tab.terminalSessionId).toBe("term_123");
    expect(tab.lastSeq).toBe(99);
  });

  test("persists terminal MCP env opt-out only in session layouts", async () => {
    resetLayout([
      terminalTab({
        title: "plain",
        mcpEnv: false,
        sessionMcpEnv: false,
        terminalSessionId: "term_plain",
        lastSeq: 7,
      }),
    ]);

    const shareable = serializeLayout();
    expect(JSON.stringify(shareable)).not.toContain("\"me\"");
    expect(JSON.stringify(shareable)).not.toContain("\"sme\"");

    const sessionSnapshot = serializeLayout({ terminalSessions: true });
    expect(JSON.stringify(sessionSnapshot)).toContain("\"me\":0");
    expect(JSON.stringify(sessionSnapshot)).toContain("\"sme\":0");

    await restoreLayout(sessionSnapshot!);

    const [tab] = activePane().tabs;
    expect(tab?.kind).toBe("terminal");
    if (tab?.kind !== "terminal") return;
    expect(tab.mcpEnv).toBe(false);
    expect(tab.sessionMcpEnv).toBe(false);
    expect(tab.terminalSessionId).toBe("term_plain");
  });

  test("persists rich prompt drafts only in session layouts", async () => {
    resetLayout([
      terminalTab({
        title: "prompt",
        richPrompt: {
          buffer: "## plan\n\nship it",
          heightPx: 420,
          open: true,
          mode: "source",
        },
      }),
    ]);

    const shareable = serializeLayout();
    expect(JSON.stringify(shareable)).not.toContain("ship it");

    const sessionSnapshot = serializeLayout({ terminalSessions: true });
    expect(JSON.stringify(sessionSnapshot)).toContain("ship it");
    expect(JSON.stringify(sessionSnapshot)).toContain("\"rph\":420");
    expect(JSON.stringify(sessionSnapshot)).toContain("\"rpo\":1");
    expect(JSON.stringify(sessionSnapshot)).toContain("\"rpm\":\"s\"");

    await restoreLayout(sessionSnapshot!);

    const [tab] = activePane().tabs;
    expect(tab?.kind).toBe("terminal");
    if (tab?.kind !== "terminal") return;
    expect(tab.richPrompt).toEqual({
      buffer: "## plan\n\nship it",
      heightPx: 420,
      open: true,
      mode: "source",
    });
  });

  test("hydrates terminal session ids onto hash-restored terminal tabs", async () => {
    resetLayout([
      terminalTab({
        title: "build",
        terminalSessionId: "term_abc",
        lastSeq: 42,
      }),
    ]);
    const sessionLayout = serializeLayout({ terminalSessions: true });
    const hashLayout = serializeLayout();

    await restoreLayout(hashLayout!);
    hydrateTerminalSessionsFromLayout(sessionLayout);

    const [tab] = activePane().tabs;
    expect(tab?.kind).toBe("terminal");
    if (tab?.kind !== "terminal") return;
    expect(tab.title).toBe("build");
    expect(tab.terminalSessionId).toBe("term_abc");
    expect(tab.lastSeq).toBe(42);
  });

  test("hydrates terminal session ids during restore before mount-time reads", async () => {
    resetLayout([
      terminalTab({
        title: "build",
        terminalSessionId: "term_pre_mount",
        lastSeq: 77,
      }),
    ]);
    const sessionLayout = serializeLayout({ terminalSessions: true });
    const hashLayout = serializeLayout();

    const restored = restoreLayout(hashLayout!, sessionLayout);
    const [tab] = activePane().tabs;
    expect(tab?.kind).toBe("terminal");
    if (tab?.kind !== "terminal") return;
    expect(tab.terminalSessionId).toBe("term_pre_mount");
    expect(tab.lastSeq).toBe(77);

    await restored;
  });
});

describe("tab labels", () => {
  test("keeps unique basenames plain", () => {
    const a = fileTab({ id: "a", path: "notes/foo.md" });
    const b = fileTab({ id: "b", path: "notes/bar.md" });
    const tabs = [a, b];

    expect(tabLabelInPane(a, tabs)).toBe("foo.md");
    expect(tabLabelInPane(b, tabs)).toBe("bar.md");
  });

  test("uses direct parent segments for shallow duplicates", () => {
    const a = fileTab({ id: "a", path: "a/foo.md" });
    const b = fileTab({ id: "b", path: "b/foo.md" });
    const tabs = [a, b];

    expect(tabLabelInPane(a, tabs)).toBe("a/foo.md");
    expect(tabLabelInPane(b, tabs)).toBe("b/foo.md");
  });

  test("drops shared prefix before choosing a divergent ancestor", () => {
    const a = fileTab({ id: "a", path: "a/x/foo.md" });
    const b = fileTab({ id: "b", path: "a/y/foo.md" });
    const tabs = [a, b];

    expect(tabLabelInPane(a, tabs)).toBe("x/foo.md");
    expect(tabLabelInPane(b, tabs)).toBe("y/foo.md");
  });

  test("drops deeper shared prefixes", () => {
    const a = fileTab({ id: "a", path: "a/x/p/foo.md" });
    const b = fileTab({ id: "b", path: "a/x/q/foo.md" });
    const tabs = [a, b];

    expect(tabLabelInPane(a, tabs)).toBe("p/foo.md");
    expect(tabLabelInPane(b, tabs)).toBe("q/foo.md");
  });

  test("collapses deeper divergent tails", () => {
    const a = fileTab({ id: "a", path: "a/x/p/foo.md" });
    const b = fileTab({ id: "b", path: "a/y/q/foo.md" });
    const tabs = [a, b];

    expect(tabLabelInPane(a, tabs)).toBe("x/[...]/foo.md");
    expect(tabLabelInPane(b, tabs)).toBe("y/[...]/foo.md");
  });

  test("re-collapses when the conflicting tab leaves the pane", () => {
    const a = fileTab({ id: "a", path: "a/foo.md" });
    const b = fileTab({ id: "b", path: "b/foo.md" });
    const tabs = [a, b];

    expect(tabLabelInPane(a, tabs)).toBe("a/foo.md");
    expect(tabLabelInPane(a, [a])).toBe("foo.md");
  });
});

describe("file tab loading", () => {
  test("focuses a loading tab before the file fetch resolves", async () => {
    resetLayout([]);
    let resolveRead: (value: Awaited<ReturnType<typeof api.read>>) => void = () => {};
    vi.spyOn(api, "read").mockReturnValue(
      new Promise((resolve) => {
        resolveRead = resolve;
      }),
    );

    const opened = openInPane(activePane().id, "notes/slow.md");
    const pane = activePane();
    const [tab] = pane.tabs;

    expect(tab?.kind).toBe("file");
    if (tab?.kind !== "file") return;
    expect(pane.activeTabId).toBe(tab.id);
    expect(tab.loading).toBe(true);
    expect(tab.content).toBe("");

    resolveRead({
      path: "notes/slow.md",
      content: "# loaded",
      mtime: 10,
      writable: true,
    });
    await opened;

    expect(tab.loading).toBe(false);
    expect(tab.content).toBe("# loaded");
    expect(tab.error).toBeNull();
  });

  test("keeps load failures inside the destination tab", async () => {
    resetLayout([]);
    vi.spyOn(api, "read").mockRejectedValue(new Error("read failed"));

    await openInPane(activePane().id, "notes/bad.md");
    const pane = activePane();
    const [tab] = pane.tabs;

    expect(tab?.kind).toBe("file");
    if (tab?.kind !== "file") return;
    expect(pane.activeTabId).toBe(tab.id);
    expect(tab.loading).toBe(false);
    expect(tab.error).toBe("read failed");
  });
});

describe("terminal tab naming", () => {
  test("opens new terminals with enumerated names", () => {
    const pane = resetLayout([
      terminalTab({ id: "term-existing", title: "Terminal-3" }),
      terminalTab({ id: "term-build", title: "build" }),
    ]);

    openTerminalInPane(pane.id);

    const created = activePane().tabs.at(-1);
    expect(created?.kind).toBe("terminal");
    if (created?.kind !== "terminal") return;
    expect(created.title).toBe("Terminal-4");
  });

  test("tracks stale CHAN_TAB_NAME after renaming a live terminal", () => {
    const tab = terminalTab({ title: "build" });
    resetLayout([tab]);

    setTerminalSession(tab, "term_live", 0, true);
    expect(tab.terminalEnvTabName).toBe("build");
    expect(terminalEnvTabNameStale(tab)).toBe(false);

    renameTerminalTab(tab, "deploy");

    expect(terminalEnvTabNameStale(tab)).toBe(true);
    expect(tab.terminalEnvNamePromptDismissed).toBe(false);

    dismissTerminalEnvNamePrompt(tab);
    expect(tab.terminalEnvNamePromptDismissed).toBe(true);

    renameTerminalTab(tab, "ship");
    expect(tab.terminalEnvNamePromptDismissed).toBe(false);

    setTerminalSession(tab, "term_new", 0, true);
    expect(tab.terminalEnvTabName).toBe("ship");
    expect(terminalEnvTabNameStale(tab)).toBe(false);
  });
});

describe("autosave", () => {
  test("strips trailing whitespace on save when the preference is enabled", async () => {
    editorToolsPrefs.stripTrailingWhitespaceOnSave = true;
    const tab = fileTab({
      content: "a  \n\tb\t\n",
      saved: "",
      savedMtime: 1,
    });
    resetLayout([tab]);
    const write = vi.spyOn(api, "write").mockResolvedValue({ mtime: 2 });

    await saveTab(tab);

    expect(write).toHaveBeenCalledWith("notes/a.md", "a\n\tb\n", 1);
    expect(tab.content).toBe("a\n\tb\n");
    expect(tab.saved).toBe("a\n\tb\n");
  });

  test("serializes overlapping saves and keeps edits after an in-flight save dirty", async () => {
    vi.useFakeTimers();
    const tab = fileTab({ content: "v1", saved: "base", savedMtime: 1 });
    const pane = resetLayout([tab]);
    const calls: string[] = [];
    const pending: Array<(value: { mtime: number }) => void> = [];
    vi.spyOn(api, "write").mockImplementation(async (_path, content) => {
      calls.push(content);
      return new Promise((resolve) => pending.push(resolve));
    });

    const firstSave = saveTab(tab);
    await Promise.resolve();
    expect(calls).toEqual(["v1"]);

    tab.content = "v2";
    scheduleAutosave(pane.id, tab.id);
    await vi.advanceTimersByTimeAsync(800);
    expect(calls).toEqual(["v1"]);

    pending.shift()!({ mtime: 2 });
    await vi.waitFor(() => expect(calls).toEqual(["v1", "v2"]));

    pending.shift()!({ mtime: 3 });
    await firstSave;
    expect(tab.saved).toBe("v2");
    expect(tab.savedMtime).toBe(3);
  });
});

describe("terminal broadcast groups", () => {
  test("target selection is isolated to the source terminal", () => {
    const a = terminalTab({ id: "term-a", title: "A" });
    const b = terminalTab({ id: "term-b", title: "B" });
    const c = terminalTab({ id: "term-c", title: "C" });
    resetLayout([a, b, c]);
    const tab = (id: string) =>
      activePane().tabs.find((candidate) => candidate.id === id) as TerminalTab;

    setTerminalBroadcastTarget(tab("term-a"), "term-b", true);

    expect(terminalBroadcastMemberIds(tab("term-a")).sort()).toEqual(["term-a", "term-b"]);
    expect(terminalBroadcastMemberIds(tab("term-b"))).toEqual([]);
    expect(tab("term-c").broadcastEnabled).toBe(false);

    setTerminalBroadcastTarget(tab("term-b"), "term-c", true);

    expect(terminalBroadcastMemberIds(tab("term-a")).sort()).toEqual(["term-a", "term-b"]);
    expect(terminalBroadcastMemberIds(tab("term-b")).sort()).toEqual(["term-b", "term-c"]);
    expect(terminalBroadcastMemberIds(tab("term-c"))).toEqual([]);
  });

  test("peer removal updates only the source and keeps mute independent", () => {
    const a = terminalTab({ id: "term-a", title: "A" });
    const b = terminalTab({ id: "term-b", title: "B" });
    const c = terminalTab({ id: "term-c", title: "C" });
    resetLayout([a, b, c]);
    const tab = (id: string) =>
      activePane().tabs.find((candidate) => candidate.id === id) as TerminalTab;

    setTerminalBroadcastTarget(tab("term-a"), "term-b", true);
    setTerminalBroadcastTarget(tab("term-a"), "term-c", true);
    setTerminalBroadcastMuted(tab("term-c"), true);

    removeTerminalFromBroadcastGroup(tab("term-a"), "term-c");

    expect(terminalBroadcastMemberIds(tab("term-a")).sort()).toEqual(["term-a", "term-b"]);
    expect(terminalBroadcastMemberIds(tab("term-b"))).toEqual([]);
    expect(tab("term-c").broadcastMuted).toBe(true);

    removeTerminalFromBroadcastGroup(tab("term-a"), "term-b");

    expect(tab("term-a").broadcastEnabled).toBe(false);
    expect(tab("term-b").broadcastEnabled).toBe(false);
    expect(tab("term-c").broadcastEnabled).toBe(false);
    expect(tab("term-c").broadcastMuted).toBe(true);
  });

  test("muted broadcast members stay in the group but skip input flow", () => {
    const a = terminalTab({ id: "term-a", title: "A" });
    const b = terminalTab({ id: "term-b", title: "B" });
    const c = terminalTab({ id: "term-c", title: "C" });
    resetLayout([a, b, c]);
    const tab = (id: string) =>
      activePane().tabs.find((candidate) => candidate.id === id) as TerminalTab;
    const received: string[] = [];
    const unregisterA = registerTerminalInputSink("term-a", (data) => received.push(`a:${data}`));
    const unregisterB = registerTerminalInputSink("term-b", (data) => received.push(`b:${data}`));
    const unregisterC = registerTerminalInputSink("term-c", (data) => received.push(`c:${data}`));

    setTerminalBroadcastTarget(tab("term-a"), "term-b", true);
    setTerminalBroadcastTarget(tab("term-a"), "term-c", true);
    setTerminalBroadcastMuted(tab("term-c"), true);

    broadcastTerminalInput(tab("term-a"), "one");
    expect(received).toEqual(["b:one"]);
    expect(terminalBroadcastMemberIds(tab("term-c"))).toEqual([]);

    setTerminalBroadcastMuted(tab("term-a"), true);
    broadcastTerminalInput(tab("term-a"), "two");
    expect(received).toEqual(["b:one"]);

    unregisterA();
    unregisterB();
    unregisterC();
  });

  test("bulk mute shortcut always toggles every terminal without changing membership", () => {
    const a = terminalTab({ id: "term-a", title: "A" });
    const b = terminalTab({ id: "term-b", title: "B" });
    const c = terminalTab({ id: "term-c", title: "C" });
    resetLayout([a, b, c]);
    const tab = (id: string) =>
      activePane().tabs.find((candidate) => candidate.id === id) as TerminalTab;

    setTerminalBroadcastTarget(tab("term-a"), "term-b", true);
    setTerminalBroadcastMuted(tab("term-b"), true);

    toggleAllTerminalBroadcastMuted();
    expect(tab("term-a").broadcastMuted).toBe(true);
    expect(tab("term-b").broadcastMuted).toBe(true);
    expect(tab("term-c").broadcastMuted).toBe(true);
    expect(terminalBroadcastMemberIds(tab("term-a")).sort()).toEqual(["term-a", "term-b"]);

    setTerminalBroadcastMuted(tab("term-b"), false);
    expect(tab("term-a").broadcastMuted).toBe(true);
    expect(tab("term-b").broadcastMuted).toBeUndefined();
    expect(tab("term-c").broadcastMuted).toBe(true);

    toggleAllTerminalBroadcastMuted();
    expect(tab("term-a").broadcastMuted).toBe(true);
    expect(tab("term-b").broadcastMuted).toBe(true);
    expect(tab("term-c").broadcastMuted).toBe(true);

    toggleAllTerminalBroadcastMuted();
    expect(tab("term-a").broadcastMuted).toBeUndefined();
    expect(tab("term-b").broadcastMuted).toBeUndefined();
    expect(tab("term-c").broadcastMuted).toBeUndefined();
    expect(terminalBroadcastMemberIds(tab("term-a")).sort()).toEqual(["term-a", "term-b"]);
  });

  test("broadcast skips target ids outside this window layout", () => {
    const a = terminalTab({
      id: "term-a",
      title: "A",
      broadcastEnabled: true,
      broadcastTargetIds: ["term-a", "term-b"],
    });
    resetLayout([a]);
    const received: string[] = [];
    const unregisterA = registerTerminalInputSink("term-a", (data) => received.push(`a:${data}`));
    // Simulates another window: a live sink id exists, but no tab with
    // that id is present in this window's layout registry.
    const unregisterB = registerTerminalInputSink("term-b", (data) => received.push(`b:${data}`));

    broadcastTerminalInput(a, "one");

    expect(received).toEqual([]);

    unregisterA();
    unregisterB();
  });
});
