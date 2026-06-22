// Component test: the Open-windows feed renders each window's status dot as a
// toggle button and flips a window's state on click. This exercises the real
// Svelte 5 runtime (a static check misses the reactive feed re-render after the
// watch push), per jsdom.

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import WindowFeed from "./WindowFeed.svelte";
import { loadLibrary } from "../state/library.svelte";

// Pin the in-memory mock as the backend so the feed renders the seed windows
// with no live server, independent of how the app composes its default backend.
// The async-import factory dodges vi.mock's hoist-over-imports trap.
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

beforeEach(async () => {
  // loadLibrary subscribes the watch; the mock pushes the seed window set
  // synchronously on subscribe, populating library.windows.
  await loadLibrary();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
});

describe("WindowFeed status dot", () => {
  it("renders dots as toggle buttons and flips a detached window on click", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    const dots = [...target.querySelectorAll("button.dot")] as HTMLButtonElement[];
    expect(dots.length).toBeGreaterThan(0);
    // The seed includes one detached window — its dot offers "Open window".
    const open = dots.find((b) => b.getAttribute("aria-label") === "Open window");
    expect(open).toBeTruthy();

    open!.click();
    // The mock flips `connected` + pushes the feed; the dot re-renders as a
    // "Hide window" toggle, so no "Open window" dot remains.
    await Promise.resolve();
    flushSync();
    expect(target.querySelector('button.dot[aria-label="Open window"]')).toBeNull();
  });
});
