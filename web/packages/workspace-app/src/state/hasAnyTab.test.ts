// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import {
  hasAnyTab,
  layout,
  openBrowserInActivePane,
  type LeafNode,
  type SplitNode,
} from "./tabs.svelte";

function emptyLeaf(id: string): LeafNode {
  return { kind: "leaf", id, tabs: [], activeTabId: null };
}

function resetToSingleLeaf(): LeafNode {
  const leaf = emptyLeaf("pane-1");
  layout.nodes = { [leaf.id]: leaf };
  layout.rootId = leaf.id;
  layout.activePaneId = leaf.id;
  return leaf;
}

afterEach(() => {
  resetToSingleLeaf();
});

describe("hasAnyTab", () => {
  it("is false for a single empty leaf (the serialize-null window)", () => {
    resetToSingleLeaf();
    expect(hasAnyTab()).toBe(false);
  });

  it("is true once any leaf holds a tab", () => {
    resetToSingleLeaf();
    openBrowserInActivePane();
    expect(hasAnyTab()).toBe(true);
  });

  it("is false for a split of two empty leaves", () => {
    const a = emptyLeaf("pane-a");
    const b = emptyLeaf("pane-b");
    const split: SplitNode = {
      kind: "split",
      id: "split-1",
      direction: "row",
      a: a.id,
      b: b.id,
      ratio: 0.5,
    };
    layout.nodes = { [split.id]: split, [a.id]: a, [b.id]: b };
    layout.rootId = split.id;
    layout.activePaneId = a.id;
    expect(hasAnyTab()).toBe(false);
  });

  it("is true when one leaf of a split holds a tab", () => {
    const a = emptyLeaf("pane-a");
    const b = emptyLeaf("pane-b");
    const split: SplitNode = {
      kind: "split",
      id: "split-1",
      direction: "row",
      a: a.id,
      b: b.id,
      ratio: 0.5,
    };
    layout.nodes = { [split.id]: split, [a.id]: a, [b.id]: b };
    layout.rootId = split.id;
    layout.activePaneId = a.id;
    openBrowserInActivePane();
    expect(hasAnyTab()).toBe(true);
  });
});
