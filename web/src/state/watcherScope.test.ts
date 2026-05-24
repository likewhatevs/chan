import { beforeEach, describe, expect, test, vi } from "vitest";
import {
  activeFbScopes,
  browserSidePanes,
  browserOverlay,
  browserSelection,
  handleDraftPromoted,
  pathInAnyScope,
  refreshTreeForPath,
  tree,
} from "./store.svelte";
import { layout, openBrowserInActivePane } from "./tabs.svelte";
import { api } from "../api/client";

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
  browserSidePanes.left = false;
  browserSidePanes.right = false;
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

  test("docked file browser contributes its scope", () => {
    tree.entries = [
      { path: "tasks", is_dir: true, is_editable_text: false, missing: false } as never,
    ];
    browserSidePanes.left = true;
    browserSelection.path = "tasks";

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

  test("ignores Drafts descendants because File Browser does not expose Drafts", async () => {
    tree.loadedDirs = { "": true, Drafts: true };
    tree.entries = [
      { path: "Drafts", is_dir: true, is_editable_text: false, missing: false } as never,
    ];
    const list = vi.spyOn(api, "list").mockResolvedValue([]);

    await refreshTreeForPath("Drafts/untitled/draft.md");

    expect(list).not.toHaveBeenCalled();
    expect(tree.entries.some((e) => e.path === "Drafts/untitled")).toBe(false);
    list.mockRestore();
  });
});

describe("Track C: draft promotion refresh", () => {
  test("refreshes and selects a promoted root file", async () => {
    tree.loadedDirs = { "": true };
    const list = vi.spyOn(api, "list").mockResolvedValue([
      {
        path: "untitled-1.md",
        is_dir: false,
        is_editable_text: true,
        missing: false,
      } as never,
    ]);

    await handleDraftPromoted("untitled-1.md");

    expect(list).toHaveBeenCalledWith("");
    expect(tree.entries.some((e) => e.path === "untitled-1.md")).toBe(true);
    expect(browserSelection.path).toBe("untitled-1.md");
    list.mockRestore();
  });

  test("loads promoted file ancestors before refreshing a nested target", async () => {
    tree.loadedDirs = { "": true };
    const list = vi.spyOn(api, "list").mockImplementation(async (dir?: string | null) => {
      if (dir === "") {
        return [
          {
            path: "notes",
            is_dir: true,
            is_editable_text: false,
            missing: false,
          } as never,
        ];
      }
      if (dir === "notes") {
        return [
          {
            path: "notes/draft.md",
            is_dir: false,
            is_editable_text: true,
            missing: false,
          } as never,
        ];
      }
      return [];
    });

    await handleDraftPromoted("notes/draft.md");

    expect(list).toHaveBeenCalledWith("");
    expect(list).toHaveBeenCalledWith("notes");
    expect(tree.entries.some((e) => e.path === "notes/draft.md")).toBe(true);
    expect(browserSelection.path).toBe("notes/draft.md");
    list.mockRestore();
  });
});
