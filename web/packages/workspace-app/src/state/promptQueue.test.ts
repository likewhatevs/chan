// @vitest-environment jsdom

import { describe, expect, test } from "vitest";
import {
  beginPendingPrompt,
  failPendingPrompt,
  resolvePendingPrompt,
  setTerminalQueueDepth,
  type TerminalTab,
} from "./tabs.svelte";

// Rich Prompt queue visibility -- the tab-level state machine the WS frame
// handler (TerminalTab.svelte) and the bubble (RichPrompt.svelte) share.
// The wire/markup shape is pinned in richPromptTerminalWiring.test.ts and
// richPromptComponent.test.ts; this exercises the store transitions.

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

describe("terminal queue depth", () => {
  test("positive depths stick; zero collapses to undefined (truthiness renders)", () => {
    const tab = terminalTab();
    setTerminalQueueDepth(tab, 3);
    expect(tab.queueDepth).toBe(3);
    setTerminalQueueDepth(tab, 1);
    expect(tab.queueDepth).toBe(1);
    setTerminalQueueDepth(tab, 0);
    expect(tab.queueDepth).toBeUndefined();
  });

  test("depth counts logical messages, so a gemini poke is one, not two", () => {
    // The server enqueues a gemini body and its bare CR as two idle-gated
    // entries but reports ONE pending message; the badge shows what the user
    // sent, not how many PTY writes it takes.
    const tab = terminalTab();
    setTerminalQueueDepth(tab, 1);
    expect(tab.queueDepth).toBe(1);
  });

  test("a drained batch is one absolute step, with no intermediate badge churn", () => {
    // Five `cs terminal write` notifications queue while the agent is busy,
    // then the whole prefix drains as ONE agent turn: the server emits one
    // `queue` frame carrying the remaining depth, never 4/3/2/1.
    const tab = terminalTab();
    const frames = [1, 2, 3, 4, 5, 0];
    const seen: (number | undefined)[] = [];
    for (const depth of frames) {
      setTerminalQueueDepth(tab, depth);
      seen.push(tab.queueDepth);
    }
    expect(seen).toEqual([1, 2, 3, 4, 5, undefined]);
  });
});

describe("a batch does not disturb a pending Rich Prompt", () => {
  test("no prompt-delivered rides a batch, so a queued bubble stays queued", () => {
    // Rich Prompt is a queue boundary and the only tagged message kind, so a
    // batch of untagged notifications emits depth alone. The bubble stays
    // locked until its OWN prompt-delivered arrives.
    const tab = terminalTab();
    beginPendingPrompt(tab, "msg-1");
    resolvePendingPrompt(tab, "msg-1", "queued", 6);
    setTerminalQueueDepth(tab, 1);
    expect(tab.pendingPrompt).toEqual({ id: "msg-1", phase: "queued", depth: 6 });
    expect(tab.queueDepth).toBe(1);

    resolvePendingPrompt(tab, "msg-1", "delivered", 0);
    setTerminalQueueDepth(tab, 0);
    expect(tab.pendingPrompt).toEqual({ id: "msg-1", phase: "delivered", depth: 0 });
    expect(tab.queueDepth).toBeUndefined();
  });
});

describe("pending prompt state machine", () => {
  test("begin -> queued (ack depth = position) -> delivered", () => {
    const tab = terminalTab();
    beginPendingPrompt(tab, "msg-1");
    expect(tab.pendingPrompt).toEqual({ id: "msg-1", phase: "sent" });

    resolvePendingPrompt(tab, "msg-1", "queued", 2);
    expect(tab.pendingPrompt).toEqual({ id: "msg-1", phase: "queued", depth: 2 });

    resolvePendingPrompt(tab, "msg-1", "delivered", 1);
    expect(tab.pendingPrompt).toEqual({ id: "msg-1", phase: "delivered", depth: 1 });
  });

  test("rejected ack (queue full) resolves without losing the id", () => {
    const tab = terminalTab();
    beginPendingPrompt(tab, "msg-1");
    resolvePendingPrompt(tab, "msg-1", "rejected", 100);
    expect(tab.pendingPrompt).toEqual({ id: "msg-1", phase: "rejected", depth: 100 });
  });

  test("stale/foreign ids no-op: another window's delivered cannot flip my pending", () => {
    const tab = terminalTab();
    beginPendingPrompt(tab, "mine");
    resolvePendingPrompt(tab, "theirs", "delivered", 0);
    expect(tab.pendingPrompt).toEqual({ id: "mine", phase: "sent" });
    // No pending at all: resolve is a no-op, not a phantom pending.
    const idle = terminalTab();
    resolvePendingPrompt(idle, "ghost", "queued", 1);
    expect(idle.pendingPrompt).toBeUndefined();
  });

  test("failPendingPrompt is unguarded (WS close has no id) but needs a pending", () => {
    const tab = terminalTab();
    beginPendingPrompt(tab, "msg-1");
    resolvePendingPrompt(tab, "msg-1", "queued", 1);
    failPendingPrompt(tab);
    expect(tab.pendingPrompt).toEqual({ id: "msg-1", phase: "failed", depth: 1 });

    const idle = terminalTab();
    failPendingPrompt(idle);
    expect(idle.pendingPrompt).toBeUndefined();
  });

  test("a new begin replaces a leftover resolved pending", () => {
    const tab = terminalTab();
    beginPendingPrompt(tab, "old");
    failPendingPrompt(tab);
    beginPendingPrompt(tab, "new");
    expect(tab.pendingPrompt).toEqual({ id: "new", phase: "sent" });
  });
});
