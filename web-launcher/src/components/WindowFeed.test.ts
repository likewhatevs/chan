// Component test: the Open-windows feed renders each window with two icon
// actions — [FOCUS] (openWindow: focus / un-hide) and [SHOW/HIDE] (toggleWindow:
// Eye visible / EyeOff hidden) — and splits visible vs hidden windows into
// "Open windows" / "Hidden windows" sections keyed on the server-persisted
// `hidden` (Theme 5). This exercises the real Svelte 5 runtime (a static check
// misses the reactive feed re-render after the watch push, e.g. the Eye↔EyeOff
// flip + the row moving between sections), per jsdom.

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

function ariaButton(label: string): HTMLButtonElement | undefined {
  return [...(target?.querySelectorAll("button[aria-label]") ?? [])].find(
    (b) => b.getAttribute("aria-label") === label,
  ) as HTMLButtonElement | undefined;
}

function headings(): string[] {
  return [...(target?.querySelectorAll(".feed-heading") ?? [])].map(
    (h) => h.textContent?.trim() ?? "",
  );
}

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

describe("WindowFeed row actions", () => {
  it("renders [FOCUS] + [SHOW/HIDE] icon buttons, no whole-row toggle, no dot", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The whole-row toggle and the status dot are gone on the mutable surface.
    expect(target.querySelector("button.row-toggle")).toBeNull();
    expect(target.querySelector(".dot")).toBeNull();

    // Every row carries a FOCUS button; the SHOW/HIDE label reflects visibility
    // (Eye→"Hide window" when visible, EyeOff→"Show window" when hidden).
    const focusBtns = [...target.querySelectorAll('button[aria-label="Focus window"]')];
    expect(focusBtns.length).toBeGreaterThan(0);
    // The seed has visible windows and one hidden window (w-ds1-term-1).
    expect(ariaButton("Hide window")).toBeTruthy();
    expect(ariaButton("Show window")).toBeTruthy();
  });

  it("splits visible vs hidden into Open windows / Hidden windows sections", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The seed carries one hidden devserver window, so both headings render in
    // order (Open above Hidden, mirroring the native Window menu).
    expect(headings()).toEqual(["Open windows", "Hidden windows"]);

    // The hidden row sits under the Hidden section and only there.
    const hiddenHeading = [...target.querySelectorAll(".feed-heading")].find(
      (h) => h.textContent?.trim() === "Hidden windows",
    )!;
    // Everything after the Hidden heading is the hidden content; the "Show window"
    // (EyeOff) toggle belongs to it.
    const show = ariaButton("Show window")!;
    expect(hiddenHeading.compareDocumentPosition(show) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("SHOW/HIDE un-hides a hidden window (EyeOff→Eye, row leaves Hidden)", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The seed includes one hidden window — its SHOW/HIDE button shows EyeOff
    // ("Show window"). Clicking it opens (un-hides) the window.
    const show = ariaButton("Show window");
    expect(show).toBeTruthy();

    show!.click();
    // The mock flips `hidden`/`connected` + pushes the feed; the row re-renders to
    // "Hide window" (Eye) under Open, so no "Show window" and no Hidden section.
    await Promise.resolve();
    flushSync();
    expect(ariaButton("Show window")).toBeUndefined();
    expect(headings()).toEqual(["Open windows"]);
  });

  it("[FOCUS] calls openWindow (focus / un-hide), not hideWindow", async () => {
    const { backend } = await import("../api/backend");
    const open = vi.spyOn(backend, "openWindow");
    const hide = vi.spyOn(backend, "hideWindow");

    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The first row is a visible local window; its FOCUS takes focus via
    // openWindow (the focus / un-hide op) and never hides.
    ariaButton("Focus window")!.click();
    await Promise.resolve();
    flushSync();
    expect(open).toHaveBeenCalledTimes(1);
    expect(hide).not.toHaveBeenCalled();

    open.mockRestore();
    hide.mockRestore();
  });

  it("pins the devserver's control terminal FIRST in its Open group (W3)", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The seed devserver "prod" group carries a control terminal (control:true,
    // ordinal 0, visible); the Open section's prod group sorts it first and
    // labels it "Control terminal". (The Open section renders before Hidden, so
    // the first "prod" group in the DOM is the Open one.)
    const groups = [...target.querySelectorAll(".group")];
    const dsGroup = groups.find((g) =>
      g.querySelector(".group-title")?.textContent?.includes("prod"),
    );
    expect(dsGroup).toBeTruthy();
    const firstRowName = dsGroup!.querySelector(".rows li .row-name");
    expect(firstRowName?.textContent?.trim()).toBe("Control terminal");
  });
});
