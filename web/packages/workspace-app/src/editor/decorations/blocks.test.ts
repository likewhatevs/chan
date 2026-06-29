// @vitest-environment jsdom

import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { describe, expect, test } from "vitest";
import { chanMarkdown } from "../markdown/grammar";
import { chanDecorations } from ".";
import blocksSource from "./blocks.ts?raw";
import wysiwygSource from "../Wysiwyg.svelte?raw";

const removedListLineHook = ["cm", "md", "list", "line"].join("-");
const removedDepthHook = ["cm", "md", "list", "depth"].join("-");
const removedGuideAttr = ["data", "list", "guides"].join("-");
const removedGuideExtension = ["list", "Guide", "Visibility"].join("");

function mountDecorated(doc: string): { parent: HTMLDivElement; view: EditorView } {
  const parent = document.createElement("div");
  document.body.appendChild(parent);
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc,
      extensions: [chanMarkdown(), chanDecorations()],
    }),
  });
  return { parent, view };
}

describe("list guide removal", () => {
  test("list markers render without line-depth decorations", () => {
    const { parent, view } = mountDecorated(
      "normal prose\n- bullet\n  - child\n1. ordered\n- [ ] task",
    );

    expect(parent.querySelector(".cm-md-ul-marker")).toBeTruthy();
    expect(parent.querySelector(".cm-md-ol-marker")).toBeTruthy();
    expect(parent.querySelector(".cm-md-task-checkbox")).toBeTruthy();
    expect(parent.querySelector(`.${removedListLineHook}`)).toBeNull();
    expect(
      Array.from(parent.querySelectorAll(".cm-line")).some((line) =>
        Array.from(line.classList).some((cls) => cls.startsWith(removedDepthHook)),
      ),
    ).toBe(false);

    view.destroy();
    parent.remove();
  });

  test("guide extension and CSS hooks are absent from source", () => {
    expect(blocksSource).not.toContain(removedListLineHook);
    expect(blocksSource).not.toContain(removedDepthHook);
    expect(wysiwygSource).not.toContain(removedListLineHook);
    expect(wysiwygSource).not.toContain(removedDepthHook);
    expect(wysiwygSource).not.toContain(removedGuideAttr);
    expect(wysiwygSource).not.toContain(removedGuideExtension);
  });

  test("top-level list text alignment is marker-level CSS, not line padding", () => {
    expect(wysiwygSource).toContain("--cm-md-list-marker-hang");
    expect(wysiwygSource).toContain("--cm-md-task-checkbox-hang");
    expect(wysiwygSource).toMatch(
      /\.cm-md-ol-marker\)[\s\S]*--cm-md-list-marker-width: 2ch/,
    );
    expect(wysiwygSource).toMatch(
      /margin-left: calc\(-1 \* var\(--cm-md-list-marker-hang\)\)/,
    );
    expect(wysiwygSource).not.toMatch(
      new RegExp(`${removedListLineHook}[\\s\\S]{0,240}padding-left`),
    );
    expect(wysiwygSource).not.toMatch(
      new RegExp(`${removedListLineHook}[\\s\\S]{0,240}text-indent`),
    );
  });
});

describe("list marker rendering (real positioned markers)", () => {
  test("markers are real positioned characters, not zero-width + ::before", () => {
    expect(blocksSource).toContain("cm-md-ul-marker");
    expect(blocksSource).toContain("cm-md-ol-marker");
    expect(wysiwygSource).toContain(".cm-md-ul-marker");
    expect(wysiwygSource).toContain(".cm-md-ol-marker");
    // `*` / `+` markers are REPLACED by a real-width glyph widget (the
    // disc/circle/square CHARACTER), so the marker is a real positioned
    // char like the hyphen `-` and ordered `1.` markers - default CM
    // cursor/click, no caret-snap. Hyphen `-` stays a styling mark.
    expect(blocksSource).toContain("class BulletGlyphWidget");
    expect(blocksSource).toContain("Decoration.replace({ widget: new BulletGlyphWidget");
    expect(blocksSource).toContain("cm-md-ul-glyph");
    expect(blocksSource).toContain("cm-md-ul-disc");
    expect(blocksSource).toContain("cm-md-ul-circle");
    expect(blocksSource).toContain("cm-md-ul-square");
    expect(blocksSource).toContain("cm-md-ul-hyphen");
    expect(wysiwygSource).toContain(".cm-md-ul-glyph");
    expect(wysiwygSource).toContain(".cm-md-ul-hyphen");
    // The old zero-width-char + ::before glyph rendering is gone (it was
    // the source of the bullet cursor/click bugs).
    expect(wysiwygSource).not.toContain(".cm-md-ul-bullet");
    expect(wysiwygSource).not.toContain(".cm-md-ul-disc::before");
  });

  test("top-level star bullet renders the disc GLYPH char; doc keeps `*`", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "* item",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    // The `*` is REPLACED by a real-width glyph widget rendering the disc
    // character. The rendered marker text is the glyph (not `*`), but the
    // DOCUMENT keeps the literal `*` (render-only replace).
    const marker = parent.querySelector(".cm-md-ul-glyph");
    expect(marker?.textContent).toBe("●");
    expect(marker?.classList.contains("cm-md-ul-disc")).toBe(true);
    expect(view.state.doc.toString()).toBe("* item");

    view.destroy();
    parent.remove();
  });

  test("`*` and `+` share the depth glyph; `-` stays a distinct dash", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    // `*` and `+` both render the depth-0 disc GLYPH (Google Docs keys the
    // glyph off depth, not the char). `-` is excluded from the cycle and
    // keeps its literal dash via a styling mark.
    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: "* star\n+ plus\n- dash",
        extensions: [chanMarkdown(), chanDecorations()],
      }),
    });

    const markers = Array.from(parent.querySelectorAll(".cm-md-ul-marker"));
    expect(markers.length).toBe(3);
    // `*` and `+` -> disc glyph widget (real ● character).
    expect(markers[0]?.classList.contains("cm-md-ul-disc")).toBe(true);
    expect(markers[0]?.textContent).toBe("●");
    expect(markers[1]?.classList.contains("cm-md-ul-disc")).toBe(true);
    expect(markers[1]?.textContent).toBe("●");
    // `-` -> literal dash (styling mark, NOT a glyph widget).
    expect(markers[2]?.classList.contains("cm-md-ul-hyphen")).toBe(true);
    expect(markers[2]?.classList.contains("cm-md-ul-glyph")).toBe(false);
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

    const markers = Array.from(parent.querySelectorAll(".cm-md-ul-glyph"));
    // depth 0 disc ●, 1 circle ○, 2 square ■, 3 wraps back to disc ●.
    expect(markers[0]?.classList.contains("cm-md-ul-disc")).toBe(true);
    expect(markers[0]?.textContent).toBe("●");
    expect(markers[1]?.classList.contains("cm-md-ul-circle")).toBe(true);
    expect(markers[1]?.textContent).toBe("○");
    expect(markers[2]?.classList.contains("cm-md-ul-square")).toBe(true);
    expect(markers[2]?.textContent).toBe("■");
    expect(markers[3]?.classList.contains("cm-md-ul-disc")).toBe(true);
    expect(markers[3]?.textContent).toBe("●");
    // The document keeps the literal source chars (render-only replace).
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
    expect(parent.querySelector(".cm-md-task-list-marker")).toBeTruthy();
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
