// @vitest-environment jsdom
//
// Smoke tests for the `fullstack-41` Ctrl+D close-tab keybind. The
// dispatcher itself lives in App.svelte; a full mount of the app is
// expensive, so this gate combines two cheaper checks:
//
// 1. Per-tab-type behavior is exercised by directly invoking the
//    underlying `closeTab` against a synthetic layout. The Ctrl+D
//    dispatcher is a thin wrapper around `closeTab(p.id,
//    p.activeTabId)`, so once we know `closeTab` closes Files /
//    Graph / Doc tabs (existing helpers do), the dispatcher is just
//    the routing decision.
//
// 2. The routing decision (terminal skipped; modal-up skipped;
//    pane-mode skipped) is verified by reading App.svelte's source
//    and asserting the guards are present. This is the same shape
//    as `paneModeKeymap.test.ts` and catches accidental drift if
//    someone "simplifies" the handler later.

import { afterEach, describe, expect, test } from "vitest";
import {
  activePane,
  cancelPaneMode,
  closeTab,
  layout,
  openBrowserInActivePane,
  openGraphInActivePane,
  type FileTab,
  type LeafNode,
  type TerminalTab,
} from "../state/tabs.svelte";
import { clearRecentlyClosedTabsForTest } from "../state/tabs.svelte";
import app from "../App.svelte?raw";

function fileTab(partial: Partial<FileTab> = {}): FileTab {
  // Mirrors the shape used by `state/tabs.test.ts`: `content` equal
  // to `saved` keeps the tab clean so the close-tab path doesn't
  // pop the unsaved-changes confirmation modal.
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

function resetLayout(tabs: Array<FileTab | TerminalTab>): LeafNode {
  const paneId = "pane-1";
  const node: LeafNode = {
    kind: "leaf",
    id: paneId,
    tabs: [...tabs],
    activeTabId: tabs[0]?.id ?? null,
  };
  layout.nodes = { [paneId]: node };
  layout.rootId = paneId;
  layout.activePaneId = paneId;
  return node;
}

afterEach(() => {
  cancelPaneMode();
  clearRecentlyClosedTabsForTest();
});

describe("Ctrl+D close-tab behaviour", () => {
  test("closes a Files tab from the active pane", async () => {
    const pane = resetLayout([]);
    const browser = openBrowserInActivePane();
    pane.activeTabId = browser.id;

    await closeTab(pane.id, browser.id);

    expect(activePane().tabs).toHaveLength(0);
  });

  test("closes a Graph tab from the active pane", async () => {
    const pane = resetLayout([]);
    const graph = openGraphInActivePane({ mode: "semantic", scopeId: "drive" });
    pane.activeTabId = graph.id;

    await closeTab(pane.id, graph.id);

    expect(activePane().tabs.find((t) => t.id === graph.id)).toBeUndefined();
  });

  test("closes a clean doc tab from the active pane", async () => {
    const tab = fileTab();
    const pane = resetLayout([tab]);

    await closeTab(pane.id, tab.id);

    expect(activePane().tabs).toHaveLength(0);
  });
});

describe("Ctrl+D dispatcher (App.svelte raw-source guards)", () => {
  test("scoped to the literal Ctrl modifier on the D physical key", () => {
    expect(app).toContain('if (!e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;');
    expect(app).toContain('if (e.code !== "KeyD") return;');
  });

  test("skips when in-house modals are open or pane mode is active", () => {
    expect(app).toContain(
      "if (promptState.open || pathPromptState.open || confirmState.open) return;",
    );
    expect(app).toContain("if (paneMode.active) return;");
  });

  test("does not intercept Ctrl+D inside a terminal tab", () => {
    expect(app).toContain('if (active.kind === "terminal") return;');
  });

  test("listener is registered on document capture to beat CodeMirror", () => {
    // CodeMirror's keymap fires at bubble; capture-phase ensures the
    // close-tab path wins over the multi-cursor default. The third
    // argument to addEventListener is `true` (capture).
    expect(app).toContain(
      'document.addEventListener("keydown", onCtrlDCapture, true)',
    );
    expect(app).toContain(
      'document.removeEventListener("keydown", onCtrlDCapture, true)',
    );
  });
});
