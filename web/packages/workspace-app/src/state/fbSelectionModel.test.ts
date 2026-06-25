// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, test } from "vitest";

import {
  browserSelection,
  fbClearSelection,
  fbSelectRange,
  fbSelectSet,
  fbSelectSingle,
  fbToggle,
} from "./store.svelte";

// The visible-row order a real File Browser would pass to the range
// helper (display order, pre-order tree walk).
const ORDER = [
  "notes",
  "notes/a.md",
  "notes/b.md",
  "notes/c.md",
  "tasks",
  "tasks/x.md",
];

beforeEach(() => {
  fbClearSelection();
  browserSelection.showWorkspace = false;
});

afterEach(() => {
  fbClearSelection();
  browserSelection.showWorkspace = false;
});

describe("FB selection model (FB1)", () => {
  test("single select sets path, paths, and anchor to the one entry", () => {
    fbSelectSingle("notes/a.md");
    expect(browserSelection.path).toBe("notes/a.md");
    expect(browserSelection.paths).toEqual(["notes/a.md"]);
    expect(browserSelection.anchor).toBe("notes/a.md");
  });

  test("single select of null clears everything", () => {
    fbSelectSingle("notes/a.md");
    fbSelectSingle(null);
    expect(browserSelection.path).toBeNull();
    expect(browserSelection.paths).toEqual([]);
    expect(browserSelection.anchor).toBeNull();
  });

  test("the active path is always a member of the multi-set", () => {
    fbSelectSingle("notes/a.md");
    fbToggle("notes/c.md");
    expect(browserSelection.paths).toContain(browserSelection.path);
    fbSelectRange("tasks/x.md", ORDER);
    expect(browserSelection.paths).toContain(browserSelection.path);
  });

  test("toggle adds an unselected entry and makes it the active cursor", () => {
    fbSelectSingle("notes/a.md");
    fbToggle("notes/c.md");
    expect(browserSelection.paths).toEqual(["notes/a.md", "notes/c.md"]);
    expect(browserSelection.path).toBe("notes/c.md");
    expect(browserSelection.anchor).toBe("notes/c.md");
  });

  test("toggle removes a selected entry and promotes the cursor", () => {
    fbSelectSingle("notes/a.md");
    fbToggle("notes/b.md");
    fbToggle("notes/c.md");
    // Remove the active entry: cursor falls back to the last remaining.
    fbToggle("notes/c.md");
    expect(browserSelection.paths).toEqual(["notes/a.md", "notes/b.md"]);
    expect(browserSelection.path).toBe("notes/b.md");
  });

  test("toggle removing a non-active entry keeps the cursor", () => {
    fbSelectSingle("notes/a.md");
    fbToggle("notes/b.md");
    fbToggle("notes/c.md"); // c is active
    fbToggle("notes/a.md"); // remove a (not active)
    expect(browserSelection.path).toBe("notes/c.md");
    expect(browserSelection.paths).toEqual(["notes/b.md", "notes/c.md"]);
  });

  test("range select covers the inclusive span between anchor and target", () => {
    fbSelectSingle("notes/a.md"); // anchor = a
    fbSelectRange("tasks", ORDER);
    expect(browserSelection.paths).toEqual([
      "notes/a.md",
      "notes/b.md",
      "notes/c.md",
      "tasks",
    ]);
    expect(browserSelection.path).toBe("tasks");
    // Anchor is preserved so a second shift gesture pivots from a.
    expect(browserSelection.anchor).toBe("notes/a.md");
  });

  test("range select is order-independent (target above anchor)", () => {
    fbSelectSingle("tasks");
    fbSelectRange("notes/a.md", ORDER);
    expect(browserSelection.paths).toEqual([
      "notes/a.md",
      "notes/b.md",
      "notes/c.md",
      "tasks",
    ]);
    expect(browserSelection.path).toBe("notes/a.md");
    expect(browserSelection.anchor).toBe("tasks");
  });

  test("successive range selects pivot from the SAME anchor (desktop semantics)", () => {
    fbSelectSingle("notes/b.md"); // anchor = b
    fbSelectRange("tasks", ORDER); // b..tasks
    expect(browserSelection.paths).toEqual([
      "notes/b.md",
      "notes/c.md",
      "tasks",
    ]);
    // Shrink back the other way; anchor stays at b.
    fbSelectRange("notes", ORDER); // notes..b
    expect(browserSelection.paths).toEqual(["notes", "notes/a.md", "notes/b.md"]);
    expect(browserSelection.anchor).toBe("notes/b.md");
  });

  test("range select with no anchor falls back to a single select", () => {
    fbClearSelection();
    fbSelectRange("notes/c.md", ORDER);
    expect(browserSelection.paths).toEqual(["notes/c.md"]);
    expect(browserSelection.path).toBe("notes/c.md");
  });

  test("range select with an off-list endpoint falls back to single", () => {
    fbSelectSingle("notes/a.md");
    fbSelectRange("ghost/missing.md", ORDER);
    expect(browserSelection.paths).toEqual(["ghost/missing.md"]);
  });

  test("select-set (select-all / rubber-band) replaces the whole set", () => {
    fbSelectSingle("notes/a.md");
    fbSelectSet([...ORDER]);
    expect(browserSelection.paths).toEqual(ORDER);
    // Default active cursor = last entry; anchor = first.
    expect(browserSelection.path).toBe("tasks/x.md");
    expect(browserSelection.anchor).toBe("notes");
  });

  test("select-set honours an explicit active cursor", () => {
    fbSelectSet([...ORDER], "notes/b.md");
    expect(browserSelection.path).toBe("notes/b.md");
    expect(browserSelection.paths).toEqual(ORDER);
  });

  test("clear empties path, paths, and anchor", () => {
    fbSelectSet([...ORDER]);
    fbClearSelection();
    expect(browserSelection.path).toBeNull();
    expect(browserSelection.paths).toEqual([]);
    expect(browserSelection.anchor).toBeNull();
  });
});
