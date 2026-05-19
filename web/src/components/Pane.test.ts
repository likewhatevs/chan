// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test } from "vitest";

import {
  cancelPaneMode,
  enterPaneMode,
  layout,
  type LeafNode,
  type TerminalTab,
} from "../state/tabs.svelte";

const mounted: Array<Record<string, any>> = [];

class TestResizeObserver {
  observe() {}
  disconnect() {}
}

globalThis.ResizeObserver = TestResizeObserver as any;
globalThis.matchMedia = ((query: string) => ({
  matches: false,
  media: query,
  onchange: null,
  addEventListener() {},
  removeEventListener() {},
  addListener() {},
  removeListener() {},
  dispatchEvent: () => false,
})) as any;
HTMLCanvasElement.prototype.getContext = (() => ({})) as any;

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
  cancelPaneMode();
});

function terminalTab(partial: Partial<TerminalTab> = {}): TerminalTab {
  return {
    kind: "terminal",
    id: "term-1",
    title: "Terminal",
    createdAt: 1,
    broadcastEnabled: false,
    broadcastTargetIds: [],
    ...partial,
  };
}

async function renderPane(pane: LeafNode, options: { paneMode?: boolean } = {}) {
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  layout.focusColor = "blue";
  if (options.paneMode ?? true) enterPaneMode();
  else cancelPaneMode();
  const target = document.createElement("div");
  document.body.append(target);
  const { default: Pane } = await import("./Pane.svelte");
  const component = mount(Pane, { target, props: { pane } });
  mounted.push(component);
  await tick();
  return target;
}

function menuLabels(): string[] {
  return [...document.body.querySelectorAll(".hamburger-menu button")]
    .map((button) =>
      [...button.querySelectorAll(".menu-row-label, span:not(.menu-row-chord)")]
        .map((span) => span.textContent?.trim() ?? "")
        .filter(Boolean)
        .join(" ")
        .trim(),
    )
    .filter(Boolean);
}

describe("Pane terminal tab activity marker", () => {
  test("renders output-since-focus marker for inactive terminal tabs", async () => {
    const active = terminalTab({ id: "term-active", title: "Active" });
    const inactive = terminalTab({
      id: "term-bg",
      title: "Background",
      terminalActivity: true,
    });
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-test",
      tabs: [active, inactive],
      activeTabId: active.id,
    };

    const target = await renderPane(pane);

    expect(
      target.querySelector('[aria-label="terminal output since last focus"]'),
    ).not.toBeNull();
  }, 15000);
});

describe("Pane right-click menus", () => {
  test("hamburger uses window-wide focus color before navigation and split actions", async () => {
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-menu",
      tabs: [terminalTab()],
      activeTabId: "term-1",
    };
    const target = await renderPane(pane, { paneMode: false });

    target.querySelector<HTMLButtonElement>(".hamburger-trigger")?.click();
    await tick();

    expect(document.body.querySelector(".menu-label span")?.textContent?.trim()).toBe(
      "Focus border colour",
    );
    // fullstack-60: pane hamburger trimmed to just Enter Pane Mode +
    // the colour swatches. Pane Mode keystrokes carry every other
    // action (next/prev pane, split, flip, close, close all).
    expect(menuLabels()).toEqual([
      "Enter Pane Mode",
      "blue",
      "green",
      "pink",
    ]);

    const pink = [...document.body.querySelectorAll<HTMLButtonElement>(".hamburger-menu button")]
      .find((button) => button.textContent?.includes("pink"));
    pink?.click();
    await tick();

    expect(target.querySelector(".pane")?.getAttribute("data-focus-color")).toBe("pink");
  }, 15000);

  test("pane hamburger no longer renders Cmd+K-canonical entries (fullstack-60)", async () => {
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-trim",
      tabs: [terminalTab()],
      activeTabId: "term-1",
    };
    const target = await renderPane(pane, { paneMode: false });

    target.querySelector<HTMLButtonElement>(".hamburger-trigger")?.click();
    await tick();

    const labels = menuLabels();
    expect(labels).not.toContain("Next pane");
    expect(labels).not.toContain("Previous pane");
    expect(labels).not.toContain("Split right");
    expect(labels).not.toContain("Split down");
    expect(labels).not.toContain("Flip Hybrid");
    expect(labels).not.toContain("Close all tabs");
    expect(labels).not.toContain("Close pane");
  }, 15000);

  test("empty pane right-click shows the welcome menu", async () => {
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-empty",
      tabs: [],
      activeTabId: null,
    };
    const target = await renderPane(pane, { paneMode: false });

    target.querySelector(".placeholder")?.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        clientX: 20,
        clientY: 20,
      }),
    );
    await tick();

    expect(menuLabels()).toEqual([
      "Files",
      "Search",
      "Graph",
      "Terminal",
      "Settings",
    ]);
  }, 15000);

  test("empty pane left-click leaves the welcome menu closed", async () => {
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-empty-leftclick",
      tabs: [],
      activeTabId: null,
    };
    const target = await renderPane(pane, { paneMode: false });

    target.querySelector(".placeholder")?.dispatchEvent(
      new MouseEvent("click", {
        bubbles: true,
        cancelable: true,
        clientX: 20,
        clientY: 20,
        button: 0,
      }),
    );
    await tick();

    // No menu should be open after a plain left-click on the
    // empty-pane background — the welcome menu is right-click only.
    // The hamburger trigger (in the tabs strip) renders its own
    // button without opening a popover, so any `.hamburger-menu`
    // node in the DOM means the welcome popover actually opened.
    expect(document.body.querySelector(".hamburger-menu")).toBeNull();
  }, 15000);

  test("loaded pane right-click keeps reload and inspector menu", async () => {
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-loaded",
      tabs: [terminalTab()],
      activeTabId: "term-1",
    };
    const target = await renderPane(pane, { paneMode: false });

    target.querySelector(".tabs")?.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        clientX: 20,
        clientY: 20,
      }),
    );
    await tick();

    expect(menuLabels()).toEqual(["Reload", "Toggle Web Inspector"]);
  }, 15000);

  test("back-side-attention indicator surfaces when back has unread (fullstack-48 phase C)", async () => {
    const front = terminalTab({ id: "front-term", title: "front" });
    const backTerm = terminalTab({
      id: "back-term",
      title: "back",
      watcher: { path: "/tmp/w", events: [], seenIds: [], unread: true },
    });
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-attn",
      tabs: [front],
      activeTabId: front.id,
      back: { tabs: [backTerm], activeTabId: backTerm.id },
    };
    const target = await renderPane(pane, { paneMode: false });

    expect(target.querySelector(".back-attention")).not.toBeNull();
  }, 15000);

  test("back-side-attention indicator stays clear when back is idle (fullstack-48 phase C)", async () => {
    const front = terminalTab({ id: "front-term", title: "front" });
    const backTerm = terminalTab({ id: "back-term", title: "back" });
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-idle-back",
      tabs: [front],
      activeTabId: front.id,
      back: { tabs: [backTerm], activeTabId: backTerm.id },
    };
    const target = await renderPane(pane, { paneMode: false });

    expect(target.querySelector(".back-attention")).toBeNull();
  }, 15000);
});
