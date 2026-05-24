// @vitest-environment jsdom

import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { describe, expect, test } from "vitest";
import {
  listDepth,
  listDepthClass,
  listLineClass,
  orderedMarkerLabel,
} from "./blocks";
import { chanMarkdown } from "../markdown/grammar";
import { chanDecorations } from ".";
import blocksSource from "./blocks.ts?raw";
import wysiwygSource from "../Wysiwyg.svelte?raw";

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

describe("bullet marker rendering", () => {
  test("renders bullet source markers as consistent glyphs in WYSIWYG", () => {
    expect(blocksSource).toContain("class BulletMarkerWidget");
    expect(blocksSource).toContain(
      "Decoration.replace({ widget: new BulletMarkerWidget() })",
    );
    expect(blocksSource).toContain("cm-md-ul-marker");
    expect(wysiwygSource).toContain(".cm-md-ul-marker");
  });

  test("replaces dash markers in the rendered editor DOM only", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "- item",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    expect(parent.querySelector(".cm-md-ul-marker")?.textContent).toBe("•");
    expect(parent.textContent).toContain("• item");
    expect(parent.textContent).not.toContain("- item");
    expect(view.state.doc.toString()).toBe("- item");

    view.destroy();
    parent.remove();
  });

  test("does not add a bullet glyph before task-list checkboxes", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "- [ ] task",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    expect(parent.querySelector(".cm-md-ul-marker")).toBeNull();
    expect(parent.querySelector(".cm-md-task-checkbox")).toBeTruthy();
    expect(view.state.doc.toString()).toBe("- [ ] task");

    view.destroy();
    parent.remove();
  });
});

describe("horizontal rule source visibility", () => {
  test("leaves --- source text visible anywhere in the document", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "one\n---\ntwo",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    expect(parent.textContent).toContain("---");
    expect(parent.querySelector(".cm-md-hr")).toBeNull();
    expect(view.state.doc.toString()).toBe("one\n---\ntwo");
    expect(wysiwygSource).not.toContain(".cm-md-hr");

    view.destroy();
    parent.remove();
  });
});
