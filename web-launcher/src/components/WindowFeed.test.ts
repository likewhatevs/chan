// Component test: the Open-windows feed renders each window as a whole-row
// toggle button (the dot is the state indicator) and flips a window's state on
// click. This exercises the real Svelte 5 runtime (a static check misses the
// reactive feed re-render after the watch push), per jsdom.

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

describe("WindowFeed row toggle", () => {
  it("renders rows as toggle buttons and flips a detached window on click", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    const rows = [...target.querySelectorAll("button.row-toggle")] as HTMLButtonElement[];
    expect(rows.length).toBeGreaterThan(0);
    // Every row carries its connection-state dot indicator.
    expect(target.querySelector("button.row-toggle .dot")).toBeTruthy();
    // The seed includes one detached window — its row offers "Open window".
    const open = rows.find((b) => b.getAttribute("aria-label") === "Open window");
    expect(open).toBeTruthy();

    open!.click();
    // The mock flips `connected` + pushes the feed; the row re-renders as a
    // "Hide window" toggle, so no "Open window" row remains.
    await Promise.resolve();
    flushSync();
    expect(target.querySelector('button.row-toggle[aria-label="Open window"]')).toBeNull();
  });

  it("pins the devserver's control terminal FIRST in its group (W3)", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The seed devserver "prod" group carries a control terminal (control:true,
    // ordinal 0); the feed sorts it first and labels it "Control terminal".
    const groups = [...target.querySelectorAll(".group")];
    const dsGroup = groups.find((g) =>
      g.querySelector(".group-title")?.textContent?.includes("prod"),
    );
    expect(dsGroup).toBeTruthy();
    const firstRowName = dsGroup!.querySelector(".rows li .row-name");
    expect(firstRowName?.textContent?.trim()).toBe("Control terminal");
  });
});
