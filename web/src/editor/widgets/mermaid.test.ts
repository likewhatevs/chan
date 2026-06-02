// @vitest-environment jsdom

import { EditorSelection, EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { describe, expect, test } from "vitest";
import { chanMarkdown } from "../markdown/grammar";
import { mermaidDecorations } from "./mermaid";
import mermaidSrc from "./mermaid.ts?raw";
import renderSrc from "../mermaid_render.ts?raw";
import blocksSrc from "../decorations/blocks.ts?raw";
import wysiwygSrc from "../Wysiwyg.svelte?raw";

const DOC = [
  "before",
  "",
  "```mermaid",
  "pie title Pets",
  '  "Dogs" : 3',
  '  "Cats" : 2',
  "```",
  "",
  "after",
].join("\n");

// An unclosed fence (still being typed): no closer ```.
const UNCLOSED = ["before", "", "```mermaid", "pie title Pets"].join("\n");

function mount(doc: string, caret?: number): {
  parent: HTMLElement;
  view: EditorView;
} {
  const parent = document.createElement("div");
  document.body.appendChild(parent);
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc,
      selection: caret !== undefined ? EditorSelection.cursor(caret) : undefined,
      // Mounting replaces the closed block with the diagram widget;
      // mermaid is not imported until renderMermaid runs, but that is an
      // async void in the widget, so the field/decoration is jsdom-safe.
      extensions: [chanMarkdown(), mermaidDecorations(() => false)],
    }),
  });
  return { parent, view };
}

describe("mermaidDecorations (cursor-render)", () => {
  test("cursor OUTSIDE a closed block renders the diagram widget", () => {
    const { parent, view } = mount(DOC, 0); // caret at "before"
    expect(parent.querySelector(".cm-md-mermaid-rendered")).toBeTruthy();
    expect(parent.querySelector(".cm-md-mermaid-diagram")).toBeTruthy();
    // The block is replaced; the raw fence text is not in the DOM.
    expect(parent.textContent).not.toContain("pie title Pets");
    expect(view.state.doc.toString()).toBe(DOC);
    view.destroy();
    parent.remove();
  });

  test("cursor INSIDE the block suppresses the widget (source editable)", () => {
    const { parent, view } = mount(DOC, DOC.indexOf("pie title"));
    expect(parent.querySelector(".cm-md-mermaid-rendered")).toBeNull();
    view.destroy();
    parent.remove();
  });

  test("an unclosed (mid-typing) block never renders", () => {
    const { parent, view } = mount(UNCLOSED, 0);
    expect(parent.querySelector(".cm-md-mermaid-rendered")).toBeNull();
    view.destroy();
    parent.remove();
  });
});

describe("mermaid wiring", () => {
  test("mermaid is dynamic-imported (never in the initial bundle)", () => {
    expect(renderSrc).toMatch(/import\("mermaid"\)/);
    expect(renderSrc).not.toMatch(/^import .* from "mermaid"/m);
  });

  test("blocks.ts is unchanged for mermaid (no special-case)", () => {
    expect(blocksSrc).not.toMatch(/mermaid/i);
  });

  test("vertical arrow keys step INTO a rendered block (no widget skip)", () => {
    // A block-replace widget has no internal lines, so ArrowUp/Down skip
    // it (atomicRanges snaps the caret past the atom). The fix is an
    // ArrowUp/ArrowDown keymap that redirects a crossing move onto the
    // block edge so scan() de-renders it. moveVertically needs real
    // layout (jsdom has none), so the behaviour is browser-verified;
    // this pins the mechanism so it can't silently drop out.
    expect(mermaidSrc).toMatch(/key:\s*"ArrowUp",\s*run:\s*stepInto\(false\)/);
    expect(mermaidSrc).toMatch(/key:\s*"ArrowDown",\s*run:\s*stepInto\(true\)/);
    expect(mermaidSrc).toMatch(/view\.moveVertically\(range, forward\)/);
    expect(mermaidSrc).toMatch(/EditorSelection\.cursor\(enter\)/);
  });

  test("reverse flip: cursor-enter ghosts the cached face and folds it out", () => {
    // The forward flip plays on widget mount; the reverse needs a ghost
    // because CM removes the widget DOM instantly on enter. Needs real
    // layout + WAAPI (jsdom has neither), so behaviour is browser-
    // verified; this pins the mechanism. The ghost rotateX-folds from 0
    // to -90 (mirror of the forward -90 -> 0) over the same duration.
    expect(mermaidSrc).toMatch(/cacheFace\(this\.source, this\.dark, res\.svg\)/);
    expect(mermaidSrc).toMatch(/function flipOutGhost/);
    expect(mermaidSrc).toMatch(/rotateX\(0deg\)/);
    expect(mermaidSrc).toMatch(/rotateX\(-90deg\)/);
    // One chokepoint for every entry path: a block leaving the rendered
    // set on a pure cursor move ghosts out.
    expect(mermaidSrc).toMatch(/if \(update\.docChanged \|\| !update\.selectionSet\) return/);
    expect(mermaidSrc).toMatch(/flipOutGhost\(update\.view, it\.from, widget\)/);
  });

  test("error locatability: failing line accented in source + actionable face", () => {
    // Errors are cached per source on render and the source line they
    // blame (openLine + N) is line-decorated while the cursor is inside
    // the block; the rendered face leads with the line number. Browser-
    // verified end to end (needs mermaid + layout); pinned here.
    expect(mermaidSrc).toMatch(/const errorCache = new Map/);
    expect(mermaidSrc).toMatch(/cm-md-mermaid-error-line/);
    // openLine + N mapping into the document.
    expect(mermaidSrc).toMatch(/info\.openLine \+ err\.line/);
    // Actionable face leads with the line number (D3).
    expect(mermaidSrc).toMatch(/Mermaid error - line \$\{res\.errorLine\}/);
    // Error cleared on a successful re-render so a fixed line stops
    // accenting.
    expect(mermaidSrc).toMatch(/cacheError\(this\.source, null\)/);
  });

  test("no button: cursor is the only trigger; theme + rotateX wired", () => {
    expect(mermaidSrc).not.toMatch(/createElement\("button"\)/);
    expect(mermaidSrc).not.toMatch(/addEventListener\("click"/);
    expect(mermaidSrc).toMatch(/renderMermaid\(this\.source, this\.dark\)/);
    // closed-fence gate + cursor-out render.
    expect(mermaidSrc).toMatch(/closeFrom === openFrom/);
    expect(wysiwygSrc).toMatch(/cm-md-mermaid-flip-in[\s\S]{1,160}rotateX/);
    expect(wysiwygSrc).toMatch(
      /mermaidDecorations\([\s\S]{1,80}effectiveHybridSurfaceTheme\("editor"\) === "dark"/,
    );
  });
});
