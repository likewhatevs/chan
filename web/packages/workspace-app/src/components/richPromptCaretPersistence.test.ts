// @vitest-environment jsdom

// Rich Prompt caret / height / focus persistence. Two levers keep the
// composer's state alive: (1) TerminalTab keeps the bubble MOUNTED across
// tab switches (visibility-hidden like the terminal body), so the live
// EditorView carries caret/selection/undo through an active-flag flip; (2)
// the caret and drag-resized height are mirrored onto the TerminalTab
// record and serialized with the per-window session payload, so a reload or
// a cross-window restore reopens the composer where the user left it.

import { describe, expect, test, vi } from "vitest";
import terminalSrc from "./TerminalTab.svelte?raw";
import richPromptSrc from "./RichPrompt.svelte?raw";
import {
  activePane,
  hydrateTerminalSessionsFromLayout,
  layout,
  restoreLayout,
  serializeLayout,
  setRichPromptCaret,
  setRichPromptHeight,
  type LeafNode,
  type TerminalTab,
} from "../state/tabs.svelte";

// The per-file caret index is a localStorage-backed store; mock it the same
// way tabs.test.ts does so importing the tabs store never touches storage.
vi.mock("../state/caretIndex");

function resetLayout(tabs: TerminalTab[]): LeafNode {
  const pane: LeafNode = {
    kind: "leaf",
    id: "pane-rp-test",
    tabs,
    activeTabId: tabs[0]?.id ?? null,
  };
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  layout.focusColor = "blue";
  return pane;
}

function terminalTab(partial: Partial<TerminalTab> = {}): TerminalTab {
  return {
    kind: "terminal",
    id: "term-rp-1",
    title: "Terminal",
    createdAt: 1,
    broadcastEnabled: false,
    broadcastTargetIds: [],
    ...partial,
  };
}

describe("caret survives a tab switch (keep-mounted bubble)", () => {
  test("TerminalTab mounts RichPrompt independent of the active flag", () => {
    // Gating the mount on `active` would destroy the bubble (and its
    // EditorView) on every tab switch and restart the caret at offset 0 on
    // remount. The bubble stays mounted like its parent terminal and is
    // hidden by the same visibility flip.
    expect(terminalSrc).toMatch(
      /\{#if isRichPromptVisible\(tab\.id\)\}\s*<RichPrompt \{tab\} \{focused\} \/>/,
    );
    expect(terminalSrc).not.toMatch(
      /\{#if active && isRichPromptVisible\(tab\.id\)\}/,
    );
  });

  test("autofocus and refocus are gated on focused, so a hidden bubble never steals the keyboard", () => {
    // Mount autofocus: same gate FileEditorTab feeds its editors.
    expect(richPromptSrc).toMatch(/autoFocus=\{focused\}/);
    expect(richPromptSrc).not.toMatch(/autoFocus=\{true\}/);
    // Switch-back refocus: pulse-driven, double-gated, selection untouched
    // (editor.focus(), not focusAt), so the caret stays where it was.
    expect(richPromptSrc).toMatch(
      /if \(!focused\) return;\s*tabFocusPulse\.value;\s*queueMicrotask\(\(\) => \{\s*if \(!focused\) return;\s*editor\?\.focus\(\);/,
    );
    // The delivered-phase reset keeps its intentional focusAt(0), but only
    // for the focused terminal.
    expect(richPromptSrc).toMatch(
      /if \(focused\) queueMicrotask\(\(\) => editor\?\.focusAt\(0\)\);/,
    );
  });

  test("the composer editor persists its caret and reads it back (FileEditorTab parity)", () => {
    expect(richPromptSrc).toMatch(/initialCaret=\{tab\.richPromptCaret \?\? null\}/);
    expect(richPromptSrc).toMatch(
      /onCaretChange=\{\(from, to\) => setRichPromptCaret\(tab, from, to\)\}/,
    );
    // Height: seeded from the persisted field, committed on drag end.
    expect(richPromptSrc).toMatch(/\$state<number \| null>\(tab\.richPromptHeight \?\? null\)/);
    expect(richPromptSrc).toMatch(
      /if \(customHeight !== null\) setRichPromptHeight\(tab, customHeight\);/,
    );
  });
});

describe("richPromptCaret / richPromptHeight round-trip the tabs store", () => {
  test("session serialization round-trips caret + height like tab.caret", async () => {
    const term = terminalTab({ richPromptDraftPath: "drafts/t1/draft.md" });
    resetLayout([term]);
    setRichPromptCaret(term, 7, 12);
    setRichPromptHeight(term, 240);

    const snapshot = serializeLayout({ terminalSessions: true });
    expect(snapshot).not.toBeNull();
    await restoreLayout(snapshot!);

    const restored = activePane().tabs[0];
    if (restored?.kind !== "terminal") throw new Error("expected terminal tab");
    expect(restored.richPromptCaret).toEqual({ from: 7, to: 12 });
    expect(restored.richPromptHeight).toBe(240);
  });

  test("a caret at offset 0 (the fresh-composer default) is omitted", () => {
    const term = terminalTab();
    resetLayout([term]);
    setRichPromptCaret(term, 0, 0);

    const snapshot = serializeLayout({ terminalSessions: true });
    if (snapshot?.k !== "l") throw new Error("expected a leaf snapshot");
    expect(snapshot.t[0]?.rpc).toBeUndefined();
    expect(snapshot.t[0]?.rph).toBeUndefined();
  });

  test("the shareable URL hash carries neither field (session payloads only)", () => {
    const term = terminalTab();
    resetLayout([term]);
    setRichPromptCaret(term, 3, 3);
    setRichPromptHeight(term, 180);

    const snapshot = serializeLayout();
    if (snapshot?.k !== "l") throw new Error("expected a leaf snapshot");
    expect(snapshot.t[0]?.rpc).toBeUndefined();
    expect(snapshot.t[0]?.rph).toBeUndefined();
  });

  test("hydrateTerminalSessionsFromLayout grafts caret + height onto a hash restore", async () => {
    // A hash reload restores the layout WITHOUT session-only fields, then
    // grafts them positionally from the per-window session payload - same
    // path that rebinds the draft (rpd).
    const term = terminalTab({ richPromptDraftPath: "drafts/t1/draft.md" });
    resetLayout([term]);
    setRichPromptCaret(term, 5, 9);
    setRichPromptHeight(term, 300);
    const sessionSnapshot = serializeLayout({ terminalSessions: true });
    const hashSnapshot = serializeLayout();

    await restoreLayout(hashSnapshot!);
    const bare = activePane().tabs[0];
    if (bare?.kind !== "terminal") throw new Error("expected terminal tab");
    expect(bare.richPromptCaret).toBeUndefined();

    hydrateTerminalSessionsFromLayout(sessionSnapshot);
    expect(bare.richPromptCaret).toEqual({ from: 5, to: 9 });
    expect(bare.richPromptHeight).toBe(300);
  });
});
