// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test, vi } from "vitest";

import { api } from "../api/client";
import { drive } from "../state/store.svelte";
import type { TerminalWatcherState } from "../state/tabs.svelte";
import BubbleOverlay from "./BubbleOverlay.svelte";

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
  drive.info = null;
  vi.restoreAllMocks();
});

async function renderOverlay(watcher: TerminalWatcherState) {
  drive.info = {
    name: "test",
    root: "/tmp/test",
    preferences: { bubble_overlay_mode: "stack" },
  } as any;
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(BubbleOverlay, {
    target,
    props: { watcher, onRefresh: vi.fn() },
  });
  mounted.push(component);
  await tick();
  return { target };
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
  const create = vi.spyOn(api, "create").mockResolvedValue(undefined);
  const move = vi.spyOn(api, "move").mockResolvedValue({
    renamed: [],
    rewritten: [],
    conflicts: [],
  });
  return { create, move };
}

describe("BubbleOverlay", () => {
  test("single-topic numbered click replies immediately with one-shot scope", async () => {
    const { create } = installReplySpies();
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
    await waitFor(() => watcher.events.length === 0);

    const reply = JSON.parse(String(create.mock.calls[0]?.[2]));
    expect(reply).toMatchObject({
      id: "s1",
      type: "survey-reply",
      answers: [{ question_index: 0, key: "1" }],
      scope_grant: "one-shot",
    });
  });

  test("number key answers the focused multi-topic tab and auto-commits when complete", async () => {
    const { create } = installReplySpies();
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
    await waitFor(() => watcher.events.length === 0);

    const reply = JSON.parse(String(create.mock.calls[0]?.[2]));
    expect(reply.answers).toEqual([
      { question_index: 0, key: "1" },
      { question_index: 1, key: "2" },
    ]);
  });
});
