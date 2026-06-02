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

function mount(caret?: number): { parent: HTMLElement; view: EditorView } {
  const parent = document.createElement("div");
  document.body.appendChild(parent);
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc: DOC,
      selection: caret !== undefined ? EditorSelection.cursor(caret) : undefined,
      // isDark=false; mounting renders only the source face (mermaid is
      // not imported until the first flip), so this stays jsdom-safe.
      extensions: [chanMarkdown(), mermaidDecorations(() => false)],
    }),
  });
  return { parent, view };
}

describe("mermaidDecorations", () => {
  test("a mermaid block renders as a flip card when the caret is outside", () => {
    const { parent, view } = mount(0); // caret at "before"
    expect(parent.querySelector(".cm-md-mermaid-card")).toBeTruthy();
    expect(parent.querySelector(".cm-md-mermaid-source")?.textContent).toContain(
      "pie title Pets",
    );
    expect(parent.querySelector(".cm-md-mermaid-flip")).toBeTruthy();
    expect(parent.querySelector(".cm-md-mermaid-copy")).toBeTruthy();
    // The card replaces the block but never mutates the document.
    expect(view.state.doc.toString()).toBe(DOC);
    view.destroy();
    parent.remove();
  });

  test("caret inside the block suppresses the card (raw source stays editable)", () => {
    const { parent, view } = mount(DOC.indexOf("pie title"));
    expect(parent.querySelector(".cm-md-mermaid-card")).toBeNull();
    view.destroy();
    parent.remove();
  });
});

describe("mermaid wiring", () => {
  test("mermaid is dynamic-imported (never in the initial bundle)", () => {
    expect(renderSrc).toMatch(/import\("mermaid"\)/);
    expect(renderSrc).not.toMatch(/^import .* from "mermaid"/m);
  });

  test("blocks.ts hands mermaid fences to the card", () => {
    expect(blocksSrc).toMatch(/fenceLang === "mermaid"/);
  });

  test("the flip toggles a class + lazy-renders; theme is fed in", () => {
    expect(mermaidSrc).toContain("cm-md-mermaid-flipped");
    expect(mermaidSrc).toMatch(/renderMermaid\(this\.source, this\.dark\)/);
    expect(wysiwygSrc).toMatch(
      /mermaidDecorations\([\s\S]{1,80}effectiveHybridSurfaceTheme\("editor"\) === "dark"/,
    );
  });
});
