// @vitest-environment jsdom
// Real RichPrompt mount + real key dispatch, covering two pending-machine
// regressions:
//   R1 - one Escape on a queued card must cancel ONCE and keep the bubble open
//        (not also hide it via the container Escape handler).
//   R2 - a delivered-while-hidden prompt must be cleared on reopen (empty
//        composer + a clear-write to disk), not restored as stale text.
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { EditorView } from "@codemirror/view";

const writeSpy = vi.fn(async (_p: string, _c: string) => ({}) as unknown);
const readMock = vi.fn(async (_p: string) => ({ content: "" }) as unknown);
const createDraftMock = vi.fn(async () => ({ path: ".Drafts/t/draft.md" }));
const sendPromptSpy = vi.fn((..._a: unknown[]) => true);
const sendCancelSpy = vi.fn((..._a: unknown[]) => {});

vi.mock("../api/client", async (orig) => {
  const actual = (await orig()) as Record<string, unknown>;
  return {
    ...actual,
    api: {
      ...(actual.api as Record<string, unknown>),
      createDraft: () => createDraftMock(),
      read: (p: string) => readMock(p),
      write: (p: string, c: string) => writeSpy(p, c),
    },
  };
});
vi.mock("../state/tabs.svelte", async (orig) => {
  const actual = (await orig()) as Record<string, unknown>;
  return {
    ...actual,
    sendPromptToTerminal: (...a: unknown[]) => sendPromptSpy(...a),
    sendCancelToTerminal: (...a: unknown[]) => sendCancelSpy(...a),
  };
});

import RichPrompt from "./RichPrompt.svelte";
import {
  isRichPromptVisible,
  showRichPromptForTab,
  richPrompt,
} from "../state/richPrompt.svelte";
import type { TerminalTab } from "../state/tabs.svelte";

const mounted: Array<Record<string, unknown>> = [];
afterEach(() => {
  for (const c of mounted.splice(0)) unmount(c);
  richPrompt.byTab = {};
});
beforeEach(() => {
  writeSpy.mockClear();
  sendCancelSpy.mockClear();
  sendPromptSpy.mockClear();
  readMock.mockResolvedValue({ content: "" } as unknown);
});

function makeTab(over: Partial<TerminalTab> = {}): TerminalTab {
  return {
    kind: "terminal",
    id: "term-1",
    title: "t",
    createdAt: 1,
    broadcastEnabled: false,
    broadcastTargetIds: [],
    ...over,
  } as TerminalTab;
}

async function mountRP(
  tab: TerminalTab,
): Promise<{ target: HTMLElement; content: HTMLElement | null }> {
  const target = document.createElement("div");
  document.body.appendChild(target);
  mounted.push(mount(RichPrompt, { target, props: { tab } }) as Record<string, unknown>);
  for (let i = 0; i < 20 && !target.querySelector(".cm-content"); i++) {
    await tick();
    await Promise.resolve();
  }
  return { target, content: target.querySelector(".cm-content") };
}

function press(el: HTMLElement, key: string, mods: Partial<KeyboardEventInit> = {}): void {
  el.dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...mods }),
  );
}

async function settle(view: EditorView, want: string): Promise<void> {
  for (let i = 0; i < 20 && view.state.doc.toString() !== want; i++) {
    await tick();
    await Promise.resolve();
  }
}

describe("R1: Escape on a queued card cancels once and keeps the bubble open", () => {
  test("one Escape drops the queued message without hiding the composer", async () => {
    // Mount already holding a queued message (the reload-rehydration path): the
    // pending phase is `sent` and the restored draft is its text, so onMount
    // seeds `lastQueued` and the card is up.
    const tab = makeTab({
      richPromptDraftPath: ".Drafts/t/draft.md",
      pendingPrompt: { id: "p1", phase: "sent" } as TerminalTab["pendingPrompt"],
    });
    showRichPromptForTab(tab.id);
    readMock.mockResolvedValue({ content: "hello agent" } as unknown);
    const { content } = await mountRP(tab);
    expect(content).not.toBeNull();
    const view = EditorView.findFromDOM(content!)!;
    await settle(view, "hello agent");

    press(content!, "Escape");
    await tick();

    expect(sendCancelSpy).toHaveBeenCalledTimes(1); // dropped once, not twice
    expect(isRichPromptVisible(tab.id)).toBe(true); // bubble kept open
  });

  test("Escape on a plain editable draft still abandons and hides", async () => {
    const tab = makeTab({ richPromptDraftPath: ".Drafts/t/draft.md" });
    showRichPromptForTab(tab.id);
    readMock.mockResolvedValue({ content: "a draft" } as unknown);
    const { content } = await mountRP(tab);
    const view = EditorView.findFromDOM(content!)!;
    await settle(view, "a draft");

    press(content!, "Escape");
    await tick();

    expect(sendCancelSpy).not.toHaveBeenCalled();
    expect(isRichPromptVisible(tab.id)).toBe(false); // abandoned + hidden
  });
});

describe("R2: delivered-while-hidden is cleared on reopen", () => {
  test("mounting with a delivered phase + stale draft clears the composer and disk", async () => {
    const tab = makeTab({
      richPromptDraftPath: ".Drafts/t/draft.md",
      pendingPrompt: { id: "p1", phase: "delivered" } as TerminalTab["pendingPrompt"],
    });
    showRichPromptForTab(tab.id);
    readMock.mockResolvedValue({ content: "STALE delivered text" } as unknown);
    const { content } = await mountRP(tab);
    for (let i = 0; i < 10; i++) {
      await tick();
      await Promise.resolve();
    }

    const doc = content ? (content.textContent ?? "") : "";
    const clearWriteCalled = writeSpy.mock.calls.some((c) => c[1] === "");
    expect(doc).toBe("");
    expect(clearWriteCalled).toBe(true);
    expect(tab.pendingPrompt).toBeUndefined();
  });
});
