import { beforeEach, describe, expect, test } from "vitest";
import {
  activeFbScopes,
  browserOverlay,
  browserSelection,
  pathInAnyScope,
  refreshTreeForPath,
  tree,
} from "./store.svelte";
import { layout, openBrowserInActivePane } from "./tabs.svelte";

// `fullstack-b-6`: the File Browser used to react to every fs
// event on the drive (the chan-server WS stream is unscoped). We
// narrow each FB's reaction to events whose path falls inside its
// own scope so unrelated drive activity stops re-rendering the
// tree.

function resetLayout() {
  // Single empty pane as the canonical starting state; the test
  // layout helpers reach into the live layout module.
  layout.nodes = {
    root: { kind: "leaf", id: "root", tabs: [], activeTabId: null, theme: undefined },
  } as never;
  layout.rootId = "root";
  layout.activePaneId = "root";
}

beforeEach(() => {
  browserOverlay.open = false;
  browserSelection.path = null;
  tree.entries = [];
  tree.loadedDirs = {};
  tree.loadingDirs = {};
  tree.dirErrors = {};
  resetLayout();
});

describe("fullstack-b-6: pathInAnyScope", () => {
  test("empty scope matches every path (drive root scope)", () => {
    expect(pathInAnyScope("crates/lib.rs", [""])).toBe(true);
    expect(pathInAnyScope("", [""])).toBe(true);
  });

  test("dir scope matches the dir itself + descendants", () => {
    expect(pathInAnyScope("tasks", ["tasks"])).toBe(true);
    expect(pathInAnyScope("tasks/foo.md", ["tasks"])).toBe(true);
    expect(pathInAnyScope("tasks/sub/bar.md", ["tasks"])).toBe(true);
  });

  test("dir scope does NOT match siblings or look-alikes", () => {
    expect(pathInAnyScope("tasks-archive/foo.md", ["tasks"])).toBe(false);
    expect(pathInAnyScope("crates/lib.rs", ["tasks"])).toBe(false);
  });

  test("any-of semantics across multiple scopes", () => {
    expect(pathInAnyScope("crates/lib.rs", ["tasks", "crates"])).toBe(true);
    expect(pathInAnyScope("docs/intro.md", ["tasks", "crates"])).toBe(false);
  });
});

describe("fullstack-b-6: activeFbScopes", () => {
  test("no open browsers → empty list", () => {
    expect(activeFbScopes()).toEqual([]);
  });

  test("overlay open with no selection → drive-root scope", () => {
    browserOverlay.open = true;
    browserSelection.path = null;
    expect(activeFbScopes()).toEqual([""]);
  });

  test("overlay open with file selection → parent-dir scope", () => {
    tree.entries = [
      { path: "tasks", is_dir: true, is_editable_text: false, missing: false } as never,
      { path: "tasks/foo.md", is_dir: false, is_editable_text: true, missing: false } as never,
    ];
    browserOverlay.open = true;
    browserSelection.path = "tasks/foo.md";
    expect(activeFbScopes()).toEqual(["tasks"]);
  });

  test("overlay open with dir selection → that dir as scope", () => {
    tree.entries = [
      { path: "tasks", is_dir: true, is_editable_text: false, missing: false } as never,
    ];
    browserOverlay.open = true;
    browserSelection.path = "tasks";
    expect(activeFbScopes()).toEqual(["tasks"]);
  });

  test("per-pane browser tab contributes its scope", () => {
    tree.entries = [
      { path: "tasks", is_dir: true, is_editable_text: false, missing: false } as never,
    ];
    const tab = openBrowserInActivePane();
    tab.selected = "tasks";
    expect(activeFbScopes()).toContain("tasks");
  });
});

describe("fullstack-b-6: refreshTreeForPath", () => {
  test("no-op when the parent dir isn't loaded", async () => {
    tree.loadedDirs = {};
    // Should not throw and should not touch tree.entries.
    await refreshTreeForPath("crates/lib.rs");
    expect(tree.entries).toEqual([]);
  });
});
