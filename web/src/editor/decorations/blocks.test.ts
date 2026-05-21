import { describe, expect, test } from "vitest";
import {
  listDepth,
  listDepthClass,
  listLineClass,
  orderedMarkerLabel,
} from "./blocks";

describe("listDepth", () => {
  test("maps top-level list lines to depth zero", () => {
    expect(listDepth("- item")).toBe(0);
    expect(listDepth("1. item")).toBe(0);
  });

  test("maps two-space indents to one visual guide level", () => {
    expect(listDepth("  - child")).toBe(1);
    expect(listDepth("    - grandchild")).toBe(2);
  });

  test("treats a tab as one two-space list indent", () => {
    expect(listDepth("\t- child")).toBe(1);
  });

  test("walks past the old 6-level cap without losing alignment", () => {
    // 14 spaces = 7 visual levels. Previously clamped to 6, which
    // caused the deep-nesting guide drift fullstack-33 fixes.
    expect(listDepth("              - level 7")).toBe(7);
    // 22 spaces = 11 levels, still depth-agnostic.
    expect(listDepth("                      - level 11")).toBe(11);
  });

  test("soft-caps pathological indentation at 20 levels", () => {
    // 80 spaces would be 40 levels uncapped; cap keeps the guide
    // width bounded and the decoration cache finite.
    expect(listDepth(" ".repeat(80) + "- deep")).toBe(20);
  });
});

describe("listDepthClass", () => {
  test("returns the stable cm-md-list-depth-N class string", () => {
    expect(listDepthClass("- item")).toBe("cm-md-list-depth-0");
    expect(listDepthClass("  - child")).toBe("cm-md-list-depth-1");
    expect(listDepthClass("              - level 7")).toBe(
      "cm-md-list-depth-7",
    );
  });
});

describe("listLineClass", () => {
  test("marks list lines that contain markdown images", () => {
    expect(listLineClass("- Step with image ![alt](pic.png)")).toContain(
      "cm-md-list-line-image",
    );
    expect(listLineClass("  ![](images/pic.png#w=200)")).toContain(
      "cm-md-list-line-image",
    );
  });

  test("does not mark ordinary list lines as image-bearing", () => {
    expect(listLineClass("- Step with [link](doc.md)")).toBe(
      "cm-md-list-line cm-md-list-depth-0",
    );
    expect(listLineClass("- Escaped \\![alt](pic.png)")).toBe(
      "cm-md-list-line cm-md-list-depth-0",
    );
  });

  test("emits a unique class per indent level past the legacy cap", () => {
    // The 20-level smoke target from fullstack-33's acceptance
    // criteria: each level renders one guide line.
    for (let depth = 0; depth <= 20; depth++) {
      const text = " ".repeat(depth * 2) + "- level";
      expect(listLineClass(text)).toBe(
        `cm-md-list-line cm-md-list-depth-${depth}`,
      );
    }
  });
});

describe("orderedMarkerLabel (fullstack-a-40 outline-style)", () => {
  test("top-level items render as single-segment markers", () => {
    expect(orderedMarkerLabel([], 1)).toBe("1.");
    expect(orderedMarkerLabel([], 5)).toBe("5.");
  });

  test("nested items concatenate the ancestor chain", () => {
    expect(orderedMarkerLabel([1], 1)).toBe("1.1.");
    expect(orderedMarkerLabel([1], 2)).toBe("1.2.");
    expect(orderedMarkerLabel([2], 3)).toBe("2.3.");
  });

  test("deep nesting carries every ancestor segment", () => {
    expect(orderedMarkerLabel([1, 2], 3)).toBe("1.2.3.");
    expect(orderedMarkerLabel([1, 1, 1], 4)).toBe("1.1.1.4.");
  });
});
