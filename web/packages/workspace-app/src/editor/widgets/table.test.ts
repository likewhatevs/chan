// @vitest-environment jsdom

import { readFileSync } from "node:fs";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { describe, expect, test } from "vitest";
import { chanMarkdown } from "../markdown/grammar";
import { tableDecorations } from "./table";

const TABLE_DOC = [
  "before",
  "",
  "| Name | Skills |",
  "|------|--------|",
  "| @@Alice | frontend |",
  "| @@Bob | syseng |",
  "",
  "after",
].join("\n");

describe("tableDecorations", () => {
  test("renders a pipe table as a block widget without throwing", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: TABLE_DOC,
        extensions: [chanMarkdown(), tableDecorations()],
      }),
    });

    expect(parent.querySelector(".cm-md-table")).toBeTruthy();
    expect(parent.textContent).toContain("@@Alice");
    expect(view.state.doc.toString()).toBe(TABLE_DOC);

    view.destroy();
    parent.remove();
  });

  test("bold in a cell renders as <strong>", () => {
    const parent = document.createElement("div");
    document.body.appendChild(parent);

    const doc = [
      "before",
      "",
      "| Name | Note |",
      "|------|------|",
      "| Alice | **bold** |",
      "",
      "after",
    ].join("\n");

    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc,
        extensions: [chanMarkdown(), tableDecorations()],
      }),
    });

    const strong = parent.querySelector(".cm-md-table td strong");
    expect(strong).toBeTruthy();
    expect(strong?.textContent).toBe("bold");

    view.destroy();
    parent.remove();
  });

  test("wide tables are contained so prose still wraps at page width", () => {
    const source = readFileSync("src/editor/Wysiwyg.svelte", "utf8");

    expect(source).toMatch(/\.cm-content\)[\s\S]{1,500}min-width: 0;/);
    expect(source).toMatch(
      /\.cm-md-table-wrap\)[\s\S]{1,300}width: 100%;[\s\S]{1,300}max-width: 100%;[\s\S]{1,300}min-width: 0;[\s\S]{1,300}overflow-x: auto;[\s\S]{1,300}contain: inline-size;/,
    );
    expect(source).toMatch(
      /\.cm-md-table\)[\s\S]{1,500}width: max-content;[\s\S]{1,200}min-width: 100%;/,
    );
  });
});
