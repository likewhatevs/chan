// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test } from "vitest";

import BubbleOverlay from "./BubbleOverlay.svelte";

// BubbleOverlay is a Wave-1 placeholder: the real reply-capable survey
// overlay is rebuilt in Wave 2. The component must stay importable +
// mountable with no props (TerminalTab.svelte mounts `<BubbleOverlay />`)
// and render nothing.

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
});

describe("BubbleOverlay placeholder", () => {
  test("mounts with no props and renders nothing", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    const component = mount(BubbleOverlay, { target, props: {} });
    mounted.push(component);
    await tick();

    expect(target.querySelector(".bubble-overlay")).toBeNull();
    expect(target.textContent).toBe("");
  });
});
