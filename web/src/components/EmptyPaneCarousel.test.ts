// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test } from "vitest";

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
});

async function renderCarousel(props: { oncontextmenu?: (e: MouseEvent) => void } = {}) {
  const target = document.createElement("div");
  document.body.append(target);
  const { default: EmptyPaneCarousel } = await import("./EmptyPaneCarousel.svelte");
  const component = mount(EmptyPaneCarousel, { target, props });
  mounted.push(component);
  await tick();
  return target;
}

describe("EmptyPaneCarousel", () => {
  test("renders the welcome slide by default with three dots", async () => {
    const target = await renderCarousel();

    expect(target.querySelectorAll(".dot-btn").length).toBe(3);
    // Slide 1 is the welcome slide — the chan-mark watermark
    // is the verbatim placeholder bit that survives the lift.
    expect(target.querySelector(".placeholder-mark")).not.toBeNull();
    expect(target.querySelector(".slide-metadata")).toBeNull();
    expect(target.querySelector(".slide-indexing")).toBeNull();
  }, 15000);

  test("clicking a dot navigates to that slide", async () => {
    const target = await renderCarousel();

    const dots = [...target.querySelectorAll<HTMLButtonElement>(".dot-btn")];
    expect(dots.length).toBe(3);

    dots[1]?.click();
    await tick();
    expect(target.querySelector(".slide-metadata")).not.toBeNull();
    expect(target.querySelector(".placeholder-mark")).toBeNull();

    dots[2]?.click();
    await tick();
    expect(target.querySelector(".slide-indexing")).not.toBeNull();

    dots[0]?.click();
    await tick();
    expect(target.querySelector(".placeholder-mark")).not.toBeNull();
  }, 15000);

  test("forwards right-click to the parent contextmenu handler", async () => {
    let received: MouseEvent | null = null;
    const target = await renderCarousel({
      oncontextmenu: (e) => {
        received = e;
        e.preventDefault();
      },
    });

    target.querySelector(".carousel")?.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        clientX: 30,
        clientY: 30,
      }),
    );
    await tick();

    expect(received).not.toBeNull();
  }, 15000);

  test("right and left arrow keys nudge the active slide", async () => {
    const target = await renderCarousel();

    const carousel = target.querySelector<HTMLDivElement>(".carousel");
    expect(carousel).not.toBeNull();

    carousel?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
    );
    await tick();
    expect(target.querySelector(".slide-metadata")).not.toBeNull();

    carousel?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }),
    );
    await tick();
    expect(target.querySelector(".placeholder-mark")).not.toBeNull();
  }, 15000);
});
