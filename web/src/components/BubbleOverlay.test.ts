// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test, vi } from "vitest";

import { api } from "../api/client";
import { drive } from "../state/store.svelte";
import type { TerminalWatcherState } from "../state/tabs.svelte";
import BubbleOverlay from "./BubbleOverlay.svelte";

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  vi.useRealTimers();
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
  drive.info = null;
  vi.restoreAllMocks();
});

async function renderOverlay(watcher: TerminalWatcherState, onWatcherDetached = vi.fn()) {
  drive.info = {
    name: "test",
    root: "/tmp/test",
    preferences: { bubble_overlay_mode: "stack" },
  } as any;
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(BubbleOverlay, {
    target,
    props: { watcher, sessionId: "term_123", onRefresh: vi.fn(), onWatcherDetached },
  });
  mounted.push(component);
  await tick();
  return { target, onWatcherDetached };
}

function buttonText(target: ParentNode, text: string): HTMLButtonElement {
  const found = [...target.querySelectorAll("button")].find((button) =>
    button.textContent?.includes(text),
  );
  if (!found) throw new Error(`button not found: ${text}`);
  return found as HTMLButtonElement;
}

async function waitFor(condition: () => boolean): Promise<void> {
  for (let i = 0; i < 20; i += 1) {
    if (condition()) return;
    await tick();
    await Promise.resolve();
  }
}

function installReplySpies() {
  const writeReply = vi.spyOn(api, "writeTerminalEventReply").mockResolvedValue(undefined);
  return { writeReply };
}

describe("BubbleOverlay", () => {
  test("single-topic numbered click replies immediately with one-shot scope", async () => {
    vi.useFakeTimers();
    const { writeReply } = installReplySpies();
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["s1"],
      unread: false,
      events: [
        {
          id: "s1",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-s1.md",
          questions: [
            {
              header: "Mode",
              text: "Pick mode",
              options: [{ key: "1", label: "Fast" }],
            },
          ],
          standing_options: [{ key: "C", label: "Check my comments first" }],
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    buttonText(target, "Fast").click();
    await tick();
    expect(watcher.events.length).toBe(1);
    await vi.advanceTimersByTimeAsync(600);
    await waitFor(() => watcher.events.length === 0);

    expect(writeReply).toHaveBeenCalledWith("term_123", {
      id: "s1",
      type: "survey-reply",
      from: "@@Alex",
      to: "@@Architect",
      answers: [{ question_index: 0, key: "1" }],
      scope_grant: "one-shot",
    });
  });

  test("single-topic survey renders vertical numbered rows with wrapping labels", async () => {
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["layout-1"],
      unread: false,
      events: [
        {
          id: "layout-1",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-layout-1.md",
          questions: [
            {
              header: "Mode",
              text: "Pick mode",
              options: [
                {
                  key: "1",
                  label: "Use the conservative option with enough detail to wrap cleanly.",
                },
                { key: "2", label: "Use the fast option." },
              ],
            },
          ],
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    const rows = target.querySelectorAll(".option-list button");
    expect(rows).toHaveLength(3);
    expect(rows[0]?.querySelector("kbd")?.textContent).toBe("1");
    expect(rows[0]?.textContent).toContain("Use the conservative option");
  });

  test("follow-up click writes async reply and keeps the bubble visible", async () => {
    const { writeReply } = installReplySpies();
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["follow-1"],
      unread: false,
      events: [
        {
          id: "follow-1",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-follow-1.md",
          questions: [
            {
              header: "Mode",
              text: "Pick mode",
              options: [{ key: "1", label: "Fast" }],
            },
          ],
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    buttonText(target, "follow up").click();
    await tick();
    await waitFor(() => writeReply.mock.calls.length === 1);

    expect(watcher.events).toHaveLength(1);
    expect(target.textContent).toContain("follow up");
    expect(writeReply).toHaveBeenCalledWith("term_123", {
      id: "follow-1",
      type: "survey-reply",
      from: "@@Alex",
      to: "@@Architect",
      answers: [],
      scope_grant: "one-shot",
      follow_up: true,
    });
  });

  test("follow-up affordance renders as an explicit button with F marker", async () => {
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["follow-button"],
      unread: false,
      events: [
        {
          id: "follow-button",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-follow-button.md",
          questions: [
            {
              header: "Mode",
              text: "Pick mode",
              options: [{ key: "1", label: "Fast" }],
            },
          ],
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    const followButton = target.querySelector(".follow-button");
    expect(followButton).toBeInstanceOf(HTMLButtonElement);
    expect(target.querySelector(".follow-link")).toBeNull();
    expect(followButton?.querySelector("kbd")?.textContent).toBe("F");
    expect(followButton?.textContent).toContain("follow up");
  });

  test("F marks the focused survey as follow-up and a later answer supersedes it", async () => {
    vi.useFakeTimers();
    const { writeReply } = installReplySpies();
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["follow-key"],
      unread: false,
      events: [
        {
          id: "follow-key",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-follow-key.md",
          questions: [
            {
              header: "Mode",
              text: "Pick mode",
              options: [{ key: "1", label: "Fast" }],
            },
          ],
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F" }));
    await waitFor(() => writeReply.mock.calls.length === 1);
    await waitFor(() => !buttonText(target, "Fast").disabled);

    buttonText(target, "Fast").click();
    await tick();
    await vi.advanceTimersByTimeAsync(600);
    await waitFor(() => writeReply.mock.calls.length === 2);
    await waitFor(() => watcher.events.length === 0);

    expect(writeReply.mock.calls).toHaveLength(2);
    expect(writeReply.mock.calls[0]?.[1]).toMatchObject({ follow_up: true, answers: [] });
    expect(writeReply.mock.calls[1]?.[1]).toEqual({
      id: "follow-key",
      type: "survey-reply",
      from: "@@Alex",
      to: "@@Architect",
      answers: [{ question_index: 0, key: "1" }],
      scope_grant: "one-shot",
    });
  });

  test("number key answers the focused multi-topic tab and auto-commits when complete", async () => {
    vi.useFakeTimers();
    const { writeReply } = installReplySpies();
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["s2"],
      unread: false,
      events: [
        {
          id: "s2",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-s2.md",
          questions: [
            {
              header: "One",
              text: "First?",
              options: [{ key: "1", label: "A" }],
            },
            {
              header: "Two",
              text: "Second?",
              options: [{ key: "2", label: "B" }],
            },
          ],
        },
      ],
    };
    await renderOverlay(watcher);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "1" }));
    await tick();
    expect(watcher.events.length).toBe(1);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "1" }));
    await tick();
    expect(watcher.events.length).toBe(1);
    await vi.advanceTimersByTimeAsync(600);
    await waitFor(() => watcher.events.length === 0);

    expect(writeReply.mock.calls[0]?.[1].answers).toEqual([
      { question_index: 0, key: "1" },
      { question_index: 1, key: "2" },
    ]);
  });

  test("oversized surveys render bounded topics and options with a truncation hint", async () => {
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["big-1"],
      unread: false,
      events: [
        {
          id: "big-1",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-big-1.md",
          questions: Array.from({ length: 5 }, (_, idx) => ({
            header: `T${idx + 1}`,
            text: `Topic ${idx + 1}?`,
            options: [
              { key: "1", label: "One" },
              { key: "2", label: "Two" },
              { key: "3", label: "Three" },
              { key: "4", label: "Four" },
            ],
          })),
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    expect(target.querySelectorAll(".topic-tabs button")).toHaveLength(4);
    expect(target.querySelectorAll(".option-list button")).toHaveLength(3);
    expect(target.textContent).toContain("1 extra topic hidden");
    expect(target.textContent).toContain("2 extra options hidden");
  });

  test("409 reply failure clears stale watcher state through callback", async () => {
    vi.spyOn(api, "writeTerminalEventReply").mockRejectedValue(new Error("409 watcher is no longer attached"));
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["s3"],
      unread: false,
      events: [
        {
          id: "s3",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-s3.md",
          questions: [
            {
              header: "Mode",
              text: "Pick mode",
              options: [{ key: "1", label: "Fast" }],
            },
          ],
        },
      ],
    };
    const { target, onWatcherDetached } = await renderOverlay(watcher);

    buttonText(target, "Fast").click();
    await waitFor(() => onWatcherDetached.mock.calls.length === 1);

    expect(watcher.error).toBe("reply failed: watcher is no longer attached");
  });

  test("survey paired with a sibling survey-reply is filtered out of the bubble queue (fullstack-a-5)", async () => {
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["already-answered", "fresh-1"],
      unread: false,
      events: [
        {
          id: "already-answered",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-already-answered.md",
          questions: [
            {
              header: "Mode",
              text: "Pick mode",
              options: [{ key: "1", label: "Fast" }],
            },
          ],
        },
        {
          id: "already-answered",
          type: "survey-reply",
          from: "@@Alex",
          to: "@@Architect",
          path: "events/event-reply-already-answered.md",
        },
        {
          id: "fresh-1",
          type: "survey",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-fresh-1.md",
          questions: [
            {
              header: "Topic",
              text: "Pick something",
              options: [{ key: "1", label: "OK" }],
            },
          ],
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    // The replied survey must not render at all; the fresh one
    // still does. The reply itself never had a body (type ===
    // "survey-reply"); ensure both originals don't both surface.
    expect(target.textContent).not.toContain("Pick mode");
    expect(target.textContent).toContain("Pick something");
    const bubbles = target.querySelectorAll(".bubble");
    expect(bubbles).toHaveLength(1);
  });

  test("pre-flight events render numbered spawn actions", async () => {
    vi.useFakeTimers();
    const { writeReply } = installReplySpies();
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["p1"],
      unread: false,
      events: [
        {
          id: "p1",
          type: "pre-flight",
          from: "@@Spawner",
          to: "@@Alex",
          path: "events/event-p1.md",
          note: "Gemini needs login. What now?",
          session: "spawn_session",
        },
      ],
    };
    const openTerminal = vi.fn();
    drive.info = {
      name: "test",
      root: "/tmp/test",
      preferences: { bubble_overlay_mode: "stack" },
    } as any;
    const target = document.createElement("div");
    document.body.append(target);
    const component = mount(BubbleOverlay, {
      target,
      props: {
        watcher,
        sessionId: "term_123",
        onRefresh: vi.fn(),
        onOpenTerminal: openTerminal,
      },
    });
    mounted.push(component);
    await tick();

    expect(target.textContent).toContain("Gemini needs login");
    buttonText(target, "Open the terminal").click();
    await tick();

    expect(openTerminal).toHaveBeenCalledWith(expect.objectContaining({ session: "spawn_session" }));
    expect(writeReply.mock.calls[0]?.[1].answers).toEqual([
      { question_index: 0, key: "open-terminal" },
    ]);
  });
});
