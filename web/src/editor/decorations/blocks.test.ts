import { describe, expect, test } from "vitest";
import { listDepthClass, listLineClass } from "./blocks";

describe("listDepthClass", () => {
  test("maps top-level list lines to depth zero", () => {
    expect(listDepthClass("- item")).toBe("cm-md-list-depth-0");
    expect(listDepthClass("1. item")).toBe("cm-md-list-depth-0");
  });

  test("maps two-space indents to one visual guide level", () => {
    expect(listDepthClass("  - child")).toBe("cm-md-list-depth-1");
    expect(listDepthClass("    - grandchild")).toBe("cm-md-list-depth-2");
  });

  test("treats a tab as one two-space list indent", () => {
    expect(listDepthClass("\t- child")).toBe("cm-md-list-depth-1");
  });

  test("caps very deep indentation at the supported guide class", () => {
    expect(listDepthClass("                - deep")).toBe("cm-md-list-depth-6");
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
});
