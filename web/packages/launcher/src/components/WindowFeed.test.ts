// Component test: the Open-windows feed renders each window with two icon
// actions — [FOCUS] (openWindow: focus / un-hide) and [SHOW/HIDE] (toggleWindow:
// Eye visible / EyeOff hidden) — and splits visible vs hidden windows into
// "Open windows" / "Hidden windows" sections keyed on the server-persisted
// `hidden` flag. This exercises the real Svelte 5 runtime (a static check
// misses the reactive feed re-render after the watch push, e.g. the Eye↔EyeOff
// flip + the row moving between sections), per jsdom.

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import WindowFeed from "./WindowFeed.svelte";
import { library, loadLibrary } from "../state/library.svelte";
import type { WindowRecord } from "../api/library";

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

  it("lists hidden windows in the single Open-windows list (no Hidden section)", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // ONE list: a single "Open windows"
    // heading, no "Hidden windows" section. The hidden seed window is still
    // listed, marked only by its EyeOff ("Show window") toggle.
    expect(headings()).toEqual(["Open windows"]);
    expect(ariaButton("Show window")).toBeTruthy();
  });

  it("SHOW/HIDE un-hides a hidden window in place (EyeOff→Eye)", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The seed includes one hidden window — its SHOW/HIDE button shows EyeOff
    // ("Show window"). Clicking it opens (un-hides) the window.
    const show = ariaButton("Show window");
    expect(show).toBeTruthy();

    show!.click();
    // The mock flips `hidden`/`connected` + pushes the feed; the row re-renders to
    // "Hide window" (Eye) in place — still the single list, no "Show window" left.
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

  it("pins the devserver's control terminal FIRST in its group", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The seed devserver "prod" group carries a control terminal (control:true,
    // ordinal 0); the feed sorts it first in the group and labels it "Control
    // terminal".
    const groups = [...target.querySelectorAll(".group")];
    const dsGroup = groups.find((g) =>
      g.querySelector(".group-title")?.textContent?.includes("prod"),
    );
    expect(dsGroup).toBeTruthy();
    const firstRowName = dsGroup!.querySelector(".rows li .row-name");
    expect(firstRowName?.textContent?.trim()).toBe("Control terminal");
  });

  it("SHOW/HIDE surfaces a genuine bridge error in the banner, not the console", async () => {
    const { backend } = await import("../api/backend");
    // A bridge op can still reject for a GENUINE failure (no desktop bridge, a
    // network error). The eye handler must catch it and report to the banner
    // (library.error), never let a floating promise reject into the console. A
    // stale/reaped window is NOT this case — it is a clean 204 (the no-op test
    // below).
    const hide = vi.spyOn(backend, "hideWindow").mockRejectedValue(new Error("no desktop bridge"));

    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // The seed has visible windows → at least one "Hide window" (Eye) toggle.
    ariaButton("Hide window")!.click();
    // Drain the microtask queue so the rejected bridge op settles through the
    // handler's catch (run → toggleWindow → hideWindow → reportError).
    await new Promise((r) => setTimeout(r, 0));
    flushSync();

    expect(hide).toHaveBeenCalledTimes(1);
    expect(library.error).toBe("no desktop bridge");

    hide.mockRestore();
  });

  it("eye click on a reaped window is a clean 204 no-op: no banner, no console", async () => {
    const { backend } = await import("../api/backend");
    // A stale/reaped window's hide replies Ok(()) -> 204, a
    // silent no-op, NOT a 409. The launcher must treat that resolved call as a
    // clean no-op — req() accepts 204, so toggleWindow resolves and the handler
    // sets no error. Pins the 409→204 contract on the launcher side.
    const hide = vi.spyOn(backend, "hideWindow").mockResolvedValue(undefined);

    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    ariaButton("Hide window")!.click();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();

    expect(hide).toHaveBeenCalledTimes(1);
    // The clear-then-resolve path leaves no banner error (run() clears on entry,
    // the resolved 204 never reports).
    expect(library.error).toBeNull();

    hide.mockRestore();
  });
});

describe("WindowFeed duplicate-key resilience", () => {
  function makeWindow(window_id: string): WindowRecord {
    return {
      window_id,
      library_id: "local",
      kind: "terminal",
      title: "🏠 Terminal Window 1",
      ordinal: 1,
      workspace_path: null,
      prefix: "t/local-1",
      token: "tok",
      persisted: true,
      connected: true,
      control: false,
    };
  }

  it("renders one row for a duplicated (library_id, window_id), not each_key_duplicate", () => {
    // Two records sharing (library_id, window_id) would throw Svelte
    // each_key_duplicate at mount and freeze the whole feed (the same-basename
    // mount-prefix collision). The feed must drop the duplicate and render once.
    library.windows = [makeWindow("w-dup"), makeWindow("w-dup")];

    target = document.createElement("div");
    document.body.appendChild(target);
    // Mount must not throw on the duplicate key.
    app = mount(WindowFeed, { target });

    expect(target.querySelectorAll(".rows li").length).toBe(1);
  });
});
