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
      focusColor: "blue",
    };

    const target = await renderPane(pane);

    expect(
      target.querySelector('[aria-label="terminal output since last focus"]'),
    ).not.toBeNull();
  }, 15000);
});

describe("Pane right-click menus", () => {
  test("empty pane right-click shows the welcome menu", async () => {
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-empty",
      tabs: [],
      activeTabId: null,
      focusColor: "blue",
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
      "Split right",
      "Split down",
      "Settings",
    ]);
  }, 15000);

  test("loaded pane right-click keeps reload and inspector menu", async () => {
    const pane: LeafNode = {
      kind: "leaf",
      id: "pane-loaded",
      tabs: [terminalTab()],
      activeTabId: "term-1",
      focusColor: "blue",
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
});
