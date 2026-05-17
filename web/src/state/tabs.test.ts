// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { confirmState, resolveConfirm } from "./confirm.svelte";
import {
  activePane,
  closeTab,
  hydrateTerminalSessionsFromLayout,
  layout,
  registerTerminalInputSink,
  restoreLayout,
  serializeLayout,
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
  resolveConfirm(false);
  resetLayout([]);
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
