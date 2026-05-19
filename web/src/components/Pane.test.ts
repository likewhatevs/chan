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

async function renderPane(pane: LeafNode) {
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  enterPaneMode();
  const target = document.createElement("div");
  document.body.append(target);
  const { default: Pane } = await import("./Pane.svelte");
  const component = mount(Pane, { target, props: { pane } });
  mounted.push(component);
  await tick();
  return target;
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
