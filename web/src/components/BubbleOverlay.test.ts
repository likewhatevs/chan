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

async function renderOverlay(
  watcher: TerminalWatcherState,
  opts: { onWatcherDetached?: ReturnType<typeof vi.fn>; onQuoteToPrompt?: (md: string) => void } = {},
) {
  const onWatcherDetached = opts.onWatcherDetached ?? vi.fn();
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
      onWatcherDetached,
      onQuoteToPrompt: opts.onQuoteToPrompt,
    },
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

  test("`-a-69` follow-up click calls onQuoteToPrompt with the survey-as-quote markdown", async () => {
    // `fullstack-a-69` rewrote follow-up: no server reply; instead
    // the button passes the survey-as-quote markdown to
    // TerminalTab via the `onQuoteToPrompt` prop, which appends
    // to `tab.richPrompt.buffer`.
    const { writeReply } = installReplySpies();
    const quoted: string[] = [];
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
          topic: "ready to ship?",
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
    const { target } = await renderOverlay(watcher, {
      onQuoteToPrompt: (md) => quoted.push(md),
    });

    buttonText(target, "follow up").click();
    await tick();

    expect(quoted).toHaveLength(1);
    expect(quoted[0]).toContain("> **ready to ship?**");
    expect(quoted[0]).toContain("> **Mode**");
    expect(quoted[0]).toContain("> Pick mode");
    expect(quoted[0]).toContain(">   - 1: Fast");
    // No server reply on follow-up — the pre-`-a-69` writeReply
    // path is gone for this affordance.
    expect(writeReply).not.toHaveBeenCalled();
    expect(watcher.events).toHaveLength(1);
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

  test("`-a-69` F key calls onQuoteToPrompt for the focused survey; subsequent answer still works", async () => {
    // `fullstack-a-69`: F is now a UI-only action — quotes the
    // survey into the rich prompt and returns. The user can
    // still answer the survey normally afterwards; the answer
    // path is unchanged.
    vi.useFakeTimers();
    const { writeReply } = installReplySpies();
    const quoted: string[] = [];
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
    const { target } = await renderOverlay(watcher, {
      onQuoteToPrompt: (md) => quoted.push(md),
    });

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F" }));
    await tick();

    expect(quoted).toHaveLength(1);
    expect(quoted[0]).toContain("> **Mode**");
    expect(quoted[0]).toContain("> Pick mode");
    // F is now UI-only; no server reply fires.
    expect(writeReply).not.toHaveBeenCalled();

    // The user can still answer the survey normally afterwards.
    buttonText(target, "Fast").click();
    await tick();
    await vi.advanceTimersByTimeAsync(600);
    await waitFor(() => writeReply.mock.calls.length === 1);
    await waitFor(() => watcher.events.length === 0);

    expect(writeReply.mock.calls).toHaveLength(1);
    expect(writeReply.mock.calls[0]?.[1]).toEqual({
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

  test("pre-flight paired with a sibling survey-reply is filtered out (fullstack-a-28)", async () => {
    // Pre-flight bubbles with a standing-option reply (e.g. the
    // auto-appended "Check my comments first") must dismiss the
    // source bubble the same way surveys do. Before -a-28 the
    // visibleEvents predicate was thought to be survey-only;
    // pinning the actual generalized contract here.
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["preflight-replied", "preflight-fresh"],
      unread: false,
      events: [
        {
          id: "preflight-replied",
          type: "pre-flight",
          from: "@@Spawner",
          to: "@@Alex",
          path: "events/pre-flight-replied.md",
          note: "Already-answered spawn note",
          session: "spawn_a",
        },
        {
          id: "preflight-replied",
          type: "survey-reply",
          from: "@@Alex",
          to: "@@Spawner",
          path: "events/event-reply-preflight-replied.md",
        },
        {
          id: "preflight-fresh",
          type: "pre-flight",
          from: "@@Spawner",
          to: "@@Alex",
          path: "events/pre-flight-fresh.md",
          note: "Fresh spawn note",
          session: "spawn_b",
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    expect(target.textContent).not.toContain("Already-answered spawn note");
    expect(target.textContent).toContain("Fresh spawn note");
    expect(target.querySelectorAll(".bubble")).toHaveLength(1);
  });

  test("poke paired with a sibling survey-reply is filtered out (fullstack-a-28)", async () => {
    // Same predicate, third source-event type. Pokes that surface
    // a standing-options reply get the same dismissal path as
    // surveys + pre-flights.
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["poke-replied", "poke-fresh"],
      unread: false,
      events: [
        {
          id: "poke-replied",
          type: "poke",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-poke-replied.md",
          note: "Already-answered poke",
        },
        {
          id: "poke-replied",
          type: "survey-reply",
          from: "@@Alex",
          to: "@@Architect",
          path: "events/event-reply-poke-replied.md",
        },
        {
          id: "poke-fresh",
          type: "poke",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-poke-fresh.md",
          note: "Fresh poke text",
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    expect(target.textContent).not.toContain("Already-answered poke");
    expect(target.textContent).toContain("Fresh poke text");
    expect(target.querySelectorAll(".bubble")).toHaveLength(1);
  });

  test("explicit Dismiss button populates dismissedIds and drops the event (fullstack-a-28)", async () => {
    // Universal escape hatch: bubbles with no reply path (poke
    // without standing options, future notification types) get a
    // Dismiss icon that mutates the per-tab `dismissedIds` set
    // AND immediately filters the event out of `watcher.events`
    // so the bubble disappears on the next reactive cycle.
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["sticky-poke"],
      unread: false,
      events: [
        {
          id: "sticky-poke",
          type: "poke",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-sticky.md",
          note: "Sticky poke text",
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    expect(target.textContent).toContain("Sticky poke text");
    const dismissButton = [...target.querySelectorAll("button")].find(
      (b) => b.getAttribute("aria-label") === "Dismiss bubble",
    );
    if (!dismissButton) throw new Error("Dismiss button missing on bubble");
    dismissButton.click();
    await tick();

    // State mutation: dismissedIds carries the id forward for
    // subsequent polls; the event itself drops from the array so
    // production-side Svelte reactivity unmounts the bubble.
    expect(watcher.dismissedIds).toEqual(["sticky-poke"]);
    expect(watcher.events).toHaveLength(0);
  });

  test("dismissedIds hides a re-emerged source event on the next poll (fullstack-a-28)", async () => {
    // Companion to the dismiss-button test: a watcher state that
    // already has `dismissedIds` populated must not render the
    // matching source event even when the server returns it
    // (source file still on disk; reply path not taken). Mounts
    // fresh so we exercise the derived predicate at first render
    // rather than relying on intra-component mutation.
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["sticky-poke"],
      unread: false,
      dismissedIds: ["sticky-poke"],
      events: [
        {
          id: "sticky-poke",
          type: "poke",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-sticky.md",
          note: "Sticky poke text",
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    expect(target.textContent ?? "").not.toContain("Sticky poke text");
    expect(target.querySelectorAll(".bubble")).toHaveLength(0);
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

  test("pre-flight bubble suppresses spinner + label when no timing data is present (fullstack-a-38)", async () => {
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["pf-no-timing"],
      unread: false,
      events: [
        {
          id: "pf-no-timing", // no 10+ digit timestamp embedded
          type: "pre-flight",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-pf-no-timing.md",
          note: "Reply dismisses pre-flight bubble",
          session: "spawn_session",
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    expect(target.textContent).toContain("Reply dismisses pre-flight bubble");
    expect(target.querySelector(".preflight-status")).toBeNull();
    expect(target.textContent).not.toMatch(/\b0:00\b/);
  });

  test("pre-flight bubble renders spinner + elapsed when topic carries a start timestamp (fullstack-a-38)", async () => {
    const startMs = Date.now() - 12_000; // 12 s ago
    const watcher: TerminalWatcherState = {
      path: "events",
      seenIds: ["pf-with-timing"],
      unread: false,
      events: [
        {
          id: "pf-with-timing",
          type: "pre-flight",
          from: "@@Architect",
          to: "@@Alex",
          path: "events/event-pf-with-timing.md",
          topic: String(startMs),
          note: "Has timing",
          session: "spawn_session",
        },
      ],
    };
    const { target } = await renderOverlay(watcher);

    const status = target.querySelector(".preflight-status");
    expect(status).not.toBeNull();
    expect(status?.textContent ?? "").toMatch(/0:\d{2}/);
  });
});
