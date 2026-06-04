// @vitest-environment jsdom

import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { describe, expect, test } from "vitest";
import { listDepth, listDepthClass, listLineClass } from "./blocks";
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

  test("walks past 6 levels without losing alignment", () => {
    // 14 spaces = 7 visual levels.
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

  test("emits a unique class per indent level up to the 20-level cap", () => {
    // Each depth level renders one guide line.
    for (let depth = 0; depth <= 20; depth++) {
      const text = " ".repeat(depth * 2) + "- level";
      expect(listLineClass(text)).toBe(
        `cm-md-list-line cm-md-list-depth-${depth}`,
      );
    }
  });
});

describe("list marker rendering (source-faithful)", () => {
  test("classes the source marker without replacing the character", () => {
    expect(blocksSource).toContain("cm-md-ul-marker");
    expect(blocksSource).toContain("cm-md-ol-marker");
    expect(wysiwygSource).toContain(".cm-md-ul-marker");
    expect(wysiwygSource).toContain(".cm-md-ol-marker");
    // List markers render as the authored character (-, *, +, 1., 2))
    // rather than being swapped for a glyph; no replace-widgets.
    expect(blocksSource).not.toContain("BulletMarkerWidget");
    expect(blocksSource).not.toContain("OrderedMarkerWidget");
    // The `*` / `+` bullet glyph keys off nesting depth (Google-Docs
    // cycle: disc -> circle -> square). The source char stays in the doc;
    // the glyph is a CSS ::before substitution in Wysiwyg.svelte.
    expect(blocksSource).toContain("cm-md-ul-disc");
    expect(blocksSource).toContain("cm-md-ul-circle");
    expect(blocksSource).toContain("cm-md-ul-square");
    expect(wysiwygSource).toContain(".cm-md-ul-disc::before");
    expect(wysiwygSource).toContain(".cm-md-ul-circle::before");
    expect(wysiwygSource).toContain(".cm-md-ul-square::before");
    // Hyphen (`-`) lists stay a distinct literal dash, NOT a depth glyph
    // (no font-size:0 + ::before substitution).
    expect(blocksSource).toContain("cm-md-ul-hyphen");
    expect(wysiwygSource).toContain(".cm-md-ul-hyphen");
  });

  test("top-level star bullet keeps the source char and the disc class", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "* item",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    // The wysiwyg glyph is a filled disc via CSS ::before, but the doc
    // + textContent stay the literal `*` (source-faithful). The disc
    // class is the CSS hook.
    const marker = parent.querySelector(".cm-md-ul-marker");
    expect(marker?.textContent).toBe("*");
    expect(marker?.classList.contains("cm-md-ul-disc")).toBe(true);
    expect(parent.textContent).toContain("* item");
    expect(view.state.doc.toString()).toBe("* item");

    view.destroy();
    parent.remove();
  });

  test("`*` and `+` share the depth glyph; `-` stays a distinct dash", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    // `*` and `+` both render the depth-0 disc (Google Docs keys the
    // glyph off depth, not the char). `-` is excluded from the cycle and
    // keeps its literal dash, marked with the distinct hyphen class.
    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "* star\n+ plus\n- dash",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    const markers = Array.from(parent.querySelectorAll(".cm-md-ul-marker"));
    expect(markers.length).toBe(3);
    expect(markers[0]?.classList.contains("cm-md-ul-disc")).toBe(true);
    expect(markers[1]?.classList.contains("cm-md-ul-disc")).toBe(true);
    // The hyphen marker is NOT a depth glyph (no font-size:0 bullet
    // class) - it carries the distinct hyphen class instead.
    expect(markers[2]?.classList.contains("cm-md-ul-hyphen")).toBe(true);
    expect(markers[2]?.classList.contains("cm-md-ul-bullet")).toBe(false);
    expect(markers[2]?.classList.contains("cm-md-ul-disc")).toBe(false);
    expect(markers[2]?.textContent).toBe("-");
    expect(view.state.doc.toString()).toBe("* star\n+ plus\n- dash");

    view.destroy();
    parent.remove();
  });

  test("hyphen list keeps the literal dash at every nesting depth", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "- l1\n  - l2\n    - l3",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    const markers = Array.from(parent.querySelectorAll(".cm-md-ul-marker"));
    // No depth cycle: hyphen lists are distinct dashes at every level.
    for (const m of markers) {
      expect(m.classList.contains("cm-md-ul-hyphen")).toBe(true);
      expect(m.classList.contains("cm-md-ul-disc")).toBe(false);
      expect(m.classList.contains("cm-md-ul-circle")).toBe(false);
      expect(m.classList.contains("cm-md-ul-square")).toBe(false);
      expect(m.textContent).toBe("-");
    }
    expect(view.state.doc.toString()).toBe("- l1\n  - l2\n    - l3");

    view.destroy();
    parent.remove();
  });

  test("star bullet glyph cycles disc -> circle -> square by depth", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "* l1\n  * l2\n    * l3\n      * l4",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    const markers = Array.from(parent.querySelectorAll(".cm-md-ul-marker"));
    // depth 0 disc, 1 circle, 2 square, 3 wraps back to disc.
    expect(markers[0]?.classList.contains("cm-md-ul-disc")).toBe(true);
    expect(markers[1]?.classList.contains("cm-md-ul-circle")).toBe(true);
    expect(markers[2]?.classList.contains("cm-md-ul-square")).toBe(true);
    expect(markers[3]?.classList.contains("cm-md-ul-disc")).toBe(true);
    // All keep their literal source char.
    expect(view.state.doc.toString()).toBe(
      "* l1\n  * l2\n    * l3\n      * l4",
    );

    view.destroy();
    parent.remove();
  });

  test("renders ordered markers as the source `1.` / `2.`", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "1. one\n2. two",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    const markers = Array.from(
      parent.querySelectorAll(".cm-md-ol-marker"),
    ).map((el) => el.textContent);
    expect(markers).toEqual(["1.", "2."]);
    expect(parent.textContent).toContain("1. one");
    expect(parent.textContent).toContain("2. two");
    expect(view.state.doc.toString()).toBe("1. one\n2. two");

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
