// @vitest-environment jsdom

// `reconcileLayout` applies a co-viewer's persisted layout snapshot onto
// the live tree in place (never remounting tabs), and refuses — as one
// atomic no-op — anything structurally diverged. These tests pin both
// halves plus the contracts the session-sync handler builds on:
// idempotence and echo round-trips.

import { afterEach, describe, expect, test, vi } from "vitest";

// The per-file caret index is a localStorage store; mock it the same way
// tabs.test.ts does so importing the module never touches real storage.
vi.mock("./caretIndex");

import {
  layout,
  paneActiveTabId,
  paneSide,
  reconcileLayout,
  serializeLayout,
  type FileTab,
  type LeafNode,
  type SerNode,
  type SplitNode,
  type Tab,
  type TerminalTab,
} from "./tabs.svelte";

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

// Both reset helpers return the LIVE nodes read back from `layout.nodes`:
// a tab or pane object pushed into `$state` is only observable through its
// Svelte proxy — in-place mutations never land on the raw pre-push object
// (see the applyGlobalTerminalName note in tabs.svelte.ts).
function resetLayout(tabs: Tab[], partial: Partial<LeafNode> = {}): LeafNode {
  const pane: LeafNode = {
    kind: "leaf",
    id: "pane-test",
    tabs,
    activeTabId: tabs[0]?.id ?? null,
    ...partial,
  };
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  layout.focusColor = "blue";
  return layout.nodes[pane.id] as LeafNode;
}

function resetSplitLayout(
  aTabs: Tab[],
  bTabs: Tab[],
  opts: { direction?: SplitNode["direction"]; ratio?: number } = {},
): { split: SplitNode; a: LeafNode; b: LeafNode } {
  const a: LeafNode = {
    kind: "leaf",
    id: "pane-a",
    tabs: aTabs,
    activeTabId: aTabs[0]?.id ?? null,
  };
  const b: LeafNode = {
    kind: "leaf",
    id: "pane-b",
    tabs: bTabs,
    activeTabId: bTabs[0]?.id ?? null,
  };
  const split: SplitNode = {
    kind: "split",
    id: "split-test",
    direction: opts.direction ?? "row",
    a: a.id,
    b: b.id,
    ratio: opts.ratio ?? 0.5,
  };
  layout.rootId = split.id;
  layout.activePaneId = a.id;
  layout.nodes = { [a.id]: a, [b.id]: b, [split.id]: split };
  layout.focusColor = "blue";
  return {
    split: layout.nodes[split.id] as SplitNode,
    a: layout.nodes[a.id] as LeafNode,
    b: layout.nodes[b.id] as LeafNode,
  };
}

/// Stable full-state fingerprint (structure + view markers) for
/// asserting that an apply or a refused apply changed nothing.
function fingerprint(): string {
  return JSON.stringify({
    layout: serializeLayout({ terminalSessions: true }),
    activePaneId: layout.activePaneId,
    perPaneActive: Object.values(layout.nodes)
      .filter((n) => n.kind === "leaf")
      .map((n) => [n.id, n.activeTabId, n.bActiveTabId ?? null]),
  });
}

afterEach(() => {
  vi.restoreAllMocks();
  resetLayout([]);
});

describe("reconcileLayout in-place subset", () => {
  test("applies a changed split ratio", () => {
    const { split } = resetSplitLayout(
      [fileTab()],
      [fileTab({ id: "file-2", path: "notes/b.md" })],
    );
    const remote: SerNode = {
      k: "s",
      d: "r",
      r: 0.7,
      a: { k: "l", t: [{ p: "notes/a.md", m: "wysiwyg" }] },
      b: { k: "l", t: [{ p: "notes/b.md", m: "wysiwyg" }] },
    };

    expect(reconcileLayout(remote)).toBe("applied");
    expect(split.ratio).toBe(0.7);
  });

  test("an absent ratio means the peer sits at the 50/50 default", () => {
    const { split } = resetSplitLayout(
      [fileTab()],
      [fileTab({ id: "file-2", path: "notes/b.md" })],
      { ratio: 0.8 },
    );
    const remote: SerNode = {
      k: "s",
      d: "r",
      a: { k: "l", t: [{ p: "notes/a.md" }] },
      b: { k: "l", t: [{ p: "notes/b.md" }] },
    };

    expect(reconcileLayout(remote)).toBe("applied");
    expect(split.ratio).toBe(0.5);
  });

  test("applies an A/B side flip and seeds the incoming side's active tab", () => {
    const bTab = terminalTab({ id: "term-b" });
    const pane = resetLayout([fileTab()], {
      bTabs: [bTab],
      bActiveTabId: null,
    });
    const remote: SerNode = {
      k: "l",
      t: [{ p: "notes/a.md" }],
      bt: [{ k: "t", n: "Terminal" }],
      sb: 1,
    };

    expect(reconcileLayout(remote)).toBe("applied");
    expect(paneSide(pane)).toBe("b");
    expect(paneActiveTabId(pane, "b")).toBe(bTab.id);
  });

  test("flips back to side A when the remote drops sb", () => {
    const pane = resetLayout([fileTab()], {
      bTabs: [terminalTab({ id: "term-b" })],
      bActiveTabId: "term-b",
      side: "b",
    });
    const remote: SerNode = {
      k: "l",
      t: [{ p: "notes/a.md" }],
      bt: [{ k: "t", n: "Terminal" }],
    };

    expect(reconcileLayout(remote)).toBe("applied");
    expect(paneSide(pane)).toBe("a");
  });

  test("applies and clears the per-pane hybrid theme override", () => {
    const pane = resetLayout([fileTab()]);

    expect(reconcileLayout({ k: "l", t: [{ p: "notes/a.md" }], ht: "d" })).toBe(
      "applied",
    );
    expect(pane.theme).toBe("dark");

    // Absence means "follow global": the override clears, it does not stick.
    expect(reconcileLayout({ k: "l", t: [{ p: "notes/a.md" }] })).toBe("applied");
    expect(pane.theme).toBeUndefined();
  });

  test("applies the window focus color, absence meaning the default", () => {
    resetLayout([fileTab()]);

    expect(reconcileLayout({ k: "l", t: [{ p: "notes/a.md" }], wc: "g" })).toBe(
      "applied",
    );
    expect(layout.focusColor).toBe("green");

    expect(reconcileLayout({ k: "l", t: [{ p: "notes/a.md" }] })).toBe("applied");
    expect(layout.focusColor).toBe("blue");
  });

  test("applies a renamed terminal title positionally", () => {
    const pane = resetLayout([terminalTab({ terminalSessionId: "ts-1" })]);
    const remote: SerNode = {
      k: "l",
      t: [{ k: "t", n: "build watch", tsid: "ts-1" }],
    };

    expect(reconcileLayout(remote)).toBe("applied");
    expect((pane.tabs[0] as TerminalTab).title).toBe("build watch");
  });

  test("an empty remote terminal title keeps the local one", () => {
    const pane = resetLayout([terminalTab({ title: "local name" })]);

    expect(reconcileLayout({ k: "l", t: [{ k: "t", n: "" }] })).toBe("applied");
    expect((pane.tabs[0] as TerminalTab).title).toBe("local name");
  });

  test("never applies remote active/focus markers", () => {
    const tabA = fileTab({ id: "file-a" });
    const tabB = fileTab({ id: "file-b", path: "notes/b.md" });
    const { a, b } = resetSplitLayout([tabA, tabB], [terminalTab()]);
    a.activeTabId = tabA.id;
    layout.activePaneId = a.id;
    // Remote marks the OTHER tab active and the OTHER pane focused.
    const remote: SerNode = {
      k: "s",
      d: "r",
      a: {
        k: "l",
        t: [{ p: "notes/a.md" }, { p: "notes/b.md", a: 1 }],
      },
      b: { k: "l", t: [{ k: "t", n: "Terminal" }], f: 1 },
    };

    expect(reconcileLayout(remote)).toBe("applied");
    expect(a.activeTabId).toBe(tabA.id);
    expect(layout.activePaneId).toBe(a.id);
    expect(b.activeTabId).not.toBeNull();
  });
});

describe("reconcileLayout structural divergence", () => {
  test("a split-vs-leaf mismatch refuses as one atomic no-op", () => {
    resetSplitLayout([fileTab()], [terminalTab()], { ratio: 0.6 });
    const before = fingerprint();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

    expect(reconcileLayout({ k: "l", t: [{ p: "notes/a.md" }], wc: "g" })).toBe(
      "diverged",
    );
    expect(fingerprint()).toBe(before);
    expect(layout.focusColor).toBe("blue");
    expect(warn).toHaveBeenCalledOnce();
  });

  test("a split direction flip diverges", () => {
    resetSplitLayout([fileTab()], [terminalTab()]);
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const remote: SerNode = {
      k: "s",
      d: "c",
      a: { k: "l", t: [{ p: "notes/a.md" }] },
      b: { k: "l", t: [{ k: "t" }] },
    };

    expect(reconcileLayout(remote)).toBe("diverged");
  });

  test("a tab count mismatch diverges", () => {
    resetLayout([fileTab()]);
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const remote: SerNode = {
      k: "l",
      t: [{ p: "notes/a.md" }, { p: "notes/new.md" }],
    };

    expect(reconcileLayout(remote)).toBe("diverged");
    expect(layout.nodes["pane-test"]).toMatchObject({ kind: "leaf" });
  });

  test("a file path mismatch diverges", () => {
    resetLayout([fileTab()]);
    vi.spyOn(console, "warn").mockImplementation(() => {});

    expect(reconcileLayout({ k: "l", t: [{ p: "notes/other.md" }] })).toBe(
      "diverged",
    );
  });

  test("a terminal session id mismatch diverges; a missing one matches positionally", () => {
    const pane = resetLayout([terminalTab({ terminalSessionId: "ts-live" })]);
    vi.spyOn(console, "warn").mockImplementation(() => {});

    expect(
      reconcileLayout({ k: "l", t: [{ k: "t", tsid: "ts-other" }] }),
    ).toBe("diverged");

    // A structure-only blob carries no tsid: positional match is accepted.
    expect(reconcileLayout({ k: "l", t: [{ k: "t", n: "renamed" }] })).toBe(
      "applied",
    );
    expect((pane.tabs[0] as TerminalTab).title).toBe("renamed");
  });

  test("a tab kind mismatch diverges, including legacy overlay kinds", () => {
    resetLayout([fileTab()]);
    vi.spyOn(console, "warn").mockImplementation(() => {});

    expect(reconcileLayout({ k: "l", t: [{ k: "t" }] })).toBe("diverged");
    expect(reconcileLayout({ k: "l", t: [{ k: "s" }] })).toBe("diverged");
  });

  test("side B tab lists are part of congruence", () => {
    resetLayout([fileTab()], { bTabs: [terminalTab()] });
    vi.spyOn(console, "warn").mockImplementation(() => {});

    // Remote has no side B tabs -> diverged.
    expect(reconcileLayout({ k: "l", t: [{ p: "notes/a.md" }] })).toBe(
      "diverged",
    );
  });
});

describe("reconcileLayout idempotence and echo", () => {
  test("applying the same remote twice is a no-op", () => {
    resetSplitLayout(
      [fileTab()],
      [terminalTab({ terminalSessionId: "ts-1" })],
    );
    const remote: SerNode = {
      k: "s",
      d: "r",
      r: 0.62,
      wc: "p",
      a: { k: "l", t: [{ p: "notes/a.md" }], ht: "l" },
      b: { k: "l", t: [{ k: "t", n: "peer name", tsid: "ts-1" }] },
    };

    expect(reconcileLayout(remote)).toBe("applied");
    const after = fingerprint();
    expect(reconcileLayout(remote)).toBe("applied");
    expect(fingerprint()).toBe(after);
  });

  test("a serialize->reconcile round-trip of the live tree changes nothing", () => {
    resetSplitLayout(
      [fileTab(), fileTab({ id: "file-2", path: "notes/b.md" })],
      [terminalTab({ terminalSessionId: "ts-1", title: "worker" })],
      { direction: "column", ratio: 0.33 },
    );
    layout.focusColor = "orange";
    const before = fingerprint();
    const snapshot = serializeLayout({ terminalSessions: true });
    expect(snapshot).not.toBeNull();

    expect(reconcileLayout(snapshot!)).toBe("applied");
    expect(fingerprint()).toBe(before);
  });
});
