// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test } from "vitest";

import { workspace } from "../state/store.svelte";
import {
  bubbleStubVisible,
  hideBubbleStub,
  showBubbleStub,
} from "../state/bubbleStub.svelte";
import BubbleOverlay from "./BubbleOverlay.svelte";

// BubbleOverlay is a frontend-only STATIC EXAMPLE. There is no
// watcher, no session id, no reply / refresh round-trip. Visibility
// is driven by the bubbleStub rune; clicking anything dismisses the
// example with no network and no filesystem.

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
  workspace.info = null;
  hideBubbleStub();
});

async function renderOverlay(mode: "stack" | "tray" = "stack") {
  workspace.info = {
    name: "test",
    root: "/tmp/test",
    preferences: { bubble_overlay_mode: mode },
  } as any;
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(BubbleOverlay, { target, props: {} });
  mounted.push(component);
  await tick();
  return { target };
}

describe("bubbleStub state", () => {
  test("showBubbleStub flips visibility on; hideBubbleStub clears it", () => {
    hideBubbleStub();
    expect(bubbleStubVisible()).toBe(false);
    showBubbleStub();
    expect(bubbleStubVisible()).toBe(true);
    hideBubbleStub();
    expect(bubbleStubVisible()).toBe(false);
  });
});

describe("BubbleOverlay static example", () => {
  test("renders nothing until showBubbleStub flips visibility", async () => {
    const { target } = await renderOverlay();
    expect(target.querySelector(".bubble-overlay")).toBeNull();

    showBubbleStub();
    await tick();
    expect(target.querySelector(".bubble-overlay")).not.toBeNull();
  });

  test("shows the single-question, multi-question, and follow-up shapes", async () => {
    const { target } = await renderOverlay();
    showBubbleStub();
    await tick();

    // Two example bubbles: one single-question survey, one
    // multi-question survey.
    expect(target.querySelectorAll(".bubble")).toHaveLength(2);
    // Single-question example prose.
    expect(target.textContent).toContain("Single-question survey");
    // Multi-question example prose + its topic tabs.
    expect(target.textContent).toContain("Multi-question survey");
    const topicTabs = target.querySelectorAll(".topic-tabs button");
    expect(topicTabs.length).toBeGreaterThanOrEqual(2);
    // Every survey bubble carries the "F" follow-up affordance.
    const followButtons = target.querySelectorAll(".follow-button");
    expect(followButtons).toHaveLength(2);
    expect(followButtons[0]?.querySelector("kbd")?.textContent).toBe("F");
    expect(followButtons[0]?.textContent).toContain("follow up");
  });

  test("numbered option rows render with their key", async () => {
    const { target } = await renderOverlay();
    showBubbleStub();
    await tick();

    const firstOption = target.querySelector(".option-list button");
    expect(firstOption?.querySelector("kbd")?.textContent).toBe("1");
  });

  test("clicking an option dismisses the example (no reply path)", async () => {
    const { target } = await renderOverlay();
    showBubbleStub();
    await tick();
    expect(bubbleStubVisible()).toBe(true);

    const option = target.querySelector(".option-list button") as HTMLButtonElement;
    option.click();
    await tick();

    expect(bubbleStubVisible()).toBe(false);
    expect(target.querySelector(".bubble-overlay")).toBeNull();
  });

  test("clicking the follow-up affordance dismisses the example", async () => {
    const { target } = await renderOverlay();
    showBubbleStub();
    await tick();

    (target.querySelector(".follow-button") as HTMLButtonElement).click();
    await tick();

    expect(bubbleStubVisible()).toBe(false);
  });

  test("clicking the Dismiss icon dismisses the example", async () => {
    const { target } = await renderOverlay();
    showBubbleStub();
    await tick();

    const dismiss = [...target.querySelectorAll("button")].find(
      (b) => b.getAttribute("aria-label") === "Dismiss bubble",
    ) as HTMLButtonElement;
    expect(dismiss).toBeTruthy();
    dismiss.click();
    await tick();

    expect(bubbleStubVisible()).toBe(false);
  });

  test("tray mode renders the collapsed chip; clicking it dismisses", async () => {
    const { target } = await renderOverlay("tray");
    showBubbleStub();
    await tick();

    const chip = target.querySelector(".tray-chip") as HTMLButtonElement;
    expect(chip).toBeTruthy();
    expect(chip.textContent).toContain("example bubble");
    // No expanded bubble list while the tray is collapsed.
    expect(target.querySelector(".bubble-list")).toBeNull();

    chip.click();
    await tick();
    expect(bubbleStubVisible()).toBe(false);
  });
});
