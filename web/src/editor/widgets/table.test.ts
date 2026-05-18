// @vitest-environment jsdom

import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { describe, expect, test } from "vitest";
import { chanMarkdown } from "../markdown/grammar";
import { tableDecorations } from "./table";

const TABLE_DOC = [
  "before",
  "",
  "| Agent | Skills |",
  "|-------|--------|",
  "| @@FullStack | webdev |",
  "| @@Systacean | syseng |",
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
    expect(parent.textContent).toContain("@@FullStack");
    expect(view.state.doc.toString()).toBe(TABLE_DOC);

    view.destroy();
    parent.remove();
  });
});
